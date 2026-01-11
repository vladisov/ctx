use anyhow::Result;
use axum::{
    Router,
    extract::ws::{Message, WebSocket, WebSocketUpgrade},
    response::Html,
    routing::get,
};
use futures_util::{SinkExt, StreamExt};
use portable_pty::{CommandBuilder, MasterPty, PtySize, native_pty_system};
use serde::Deserialize;
use std::io::{Read, Write};
use std::sync::Arc;
use std::thread;
use tokio::net::TcpListener;
use tokio::sync::mpsc;

pub async fn handle(port: u16, data_dir: Option<&std::path::Path>) -> Result<()> {
    let data_dir_arg = data_dir.map(|p| p.to_path_buf());

    let app = Router::new().route("/", get(serve_html)).route(
        "/ws",
        get(move |ws: WebSocketUpgrade| {
            let data_dir = data_dir_arg.clone();
            async move { ws.on_upgrade(move |socket| handle_websocket(socket, data_dir)) }
        }),
    );

    let addr = format!("127.0.0.1:{}", port);
    let listener = TcpListener::bind(&addr).await?;
    let url = format!("http://{}", addr);

    println!("Web UI available at {}", url);

    // Try to open browser
    #[cfg(target_os = "macos")]
    let _ = std::process::Command::new("open").arg(&url).spawn();

    #[cfg(target_os = "linux")]
    let _ = std::process::Command::new("xdg-open").arg(&url).spawn();

    #[cfg(target_os = "windows")]
    let _ = std::process::Command::new("cmd")
        .args(["/C", "start", &url])
        .spawn();

    axum::serve(listener, app).await?;

    Ok(())
}

async fn serve_html() -> Html<&'static str> {
    Html(include_str!("web_ui.html"))
}

async fn handle_websocket(socket: WebSocket, data_dir: Option<std::path::PathBuf>) {
    if let Err(e) = run_pty_session(socket, data_dir).await {
        tracing::error!("PTY session error: {}", e);
    }
}

struct PtyHandle {
    master: Box<dyn MasterPty + Send>,
    writer: Box<dyn Write + Send>,
}

async fn run_pty_session(socket: WebSocket, data_dir: Option<std::path::PathBuf>) -> Result<()> {
    let pty_system = native_pty_system();

    // Create PTY with default size (will be resized on first message)
    let pair = pty_system.openpty(PtySize {
        rows: 24,
        cols: 80,
        pixel_width: 0,
        pixel_height: 0,
    })?;

    // Build command to run ctx ui
    let exe = std::env::current_exe()?;
    let mut cmd = CommandBuilder::new(&exe);
    cmd.arg("ui");

    if let Some(dir) = &data_dir {
        cmd.arg("--data-dir");
        cmd.arg(dir);
    }

    // Spawn the TUI in the PTY
    let _child = pair.slave.spawn_command(cmd)?;

    // Get reader/writer for the PTY
    let mut reader = pair.master.try_clone_reader()?;
    let writer = pair.master.take_writer()?;

    let pty_handle = Arc::new(std::sync::Mutex::new(PtyHandle {
        master: pair.master,
        writer,
    }));

    // Split the websocket
    let (mut ws_sender, mut ws_receiver) = socket.split();

    // Channel for PTY output -> WebSocket
    let (tx, mut rx) = mpsc::channel::<Vec<u8>>(256);

    // Spawn thread to read from PTY
    thread::spawn(move || {
        let mut buf = [0u8; 4096];
        loop {
            match reader.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => {
                    if tx.blocking_send(buf[..n].to_vec()).is_err() {
                        break;
                    }
                }
                Err(_) => break,
            }
        }
    });

    // Spawn task to forward PTY output to WebSocket
    let send_task = tokio::spawn(async move {
        while let Some(data) = rx.recv().await {
            if ws_sender.send(Message::Binary(data)).await.is_err() {
                break;
            }
        }
    });

    // Handle incoming WebSocket messages (input + resize)
    let handle_clone = Arc::clone(&pty_handle);
    let recv_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = ws_receiver.next().await {
            match msg {
                Message::Text(text) => {
                    // Check if it's a resize command
                    if let Ok(resize) = serde_json::from_str::<ResizeMessage>(&text)
                        && resize.msg_type == "resize"
                    {
                        if let Ok(h) = handle_clone.lock() {
                            let _ = h.master.resize(PtySize {
                                rows: resize.rows,
                                cols: resize.cols,
                                pixel_width: 0,
                                pixel_height: 0,
                            });
                        }
                    } else {
                        // Regular input
                        if let Ok(mut h) = handle_clone.lock() {
                            let _ = h.writer.write_all(text.as_bytes());
                        }
                    }
                }
                Message::Binary(data) => {
                    if let Ok(mut h) = handle_clone.lock() {
                        let _ = h.writer.write_all(&data);
                    }
                }
                Message::Close(_) => break,
                _ => {}
            }
        }
    });

    // Wait for either task to complete
    tokio::select! {
        _ = send_task => {}
        _ = recv_task => {}
    }

    Ok(())
}

#[derive(Deserialize)]
struct ResizeMessage {
    #[serde(rename = "type")]
    msg_type: String,
    cols: u16,
    rows: u16,
}
