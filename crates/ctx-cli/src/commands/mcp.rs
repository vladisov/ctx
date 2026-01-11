use anyhow::{Result, bail};
use ctx_mcp::{McpServer, run_stdio};
use ctx_storage::Storage;
use std::process::{Child, Command};
use std::sync::Arc;
use std::time::Duration;

pub async fn handle(
    storage: &Storage,
    host: String,
    port: u16,
    read_only: bool,
    tunnel: bool,
) -> Result<()> {
    let db = Arc::new(storage.clone());

    let _ngrok_process = if tunnel {
        Some(start_ngrok_tunnel(port).await?)
    } else {
        None
    };

    eprintln!("Starting MCP server on {}:{}", host, port);
    McpServer::serve(db, &host, port, read_only).await?;
    Ok(())
}

pub async fn handle_stdio(storage: &Storage, read_only: bool) -> Result<()> {
    let db = Arc::new(storage.clone());
    run_stdio(db, read_only).await?;
    Ok(())
}

async fn start_ngrok_tunnel(port: u16) -> Result<Child> {
    // Check if ngrok is installed
    if Command::new("ngrok").arg("--version").output().is_err() {
        bail!("ngrok not found. Install it from https://ngrok.com/download");
    }

    eprintln!("Starting ngrok tunnel...");

    // Start ngrok
    let child = Command::new("ngrok")
        .args(["http", &port.to_string(), "--log", "stderr"])
        .spawn()?;

    // Wait for ngrok to start
    tokio::time::sleep(Duration::from_secs(2)).await;

    // Query ngrok API for public URL
    match get_ngrok_url().await {
        Ok(url) => {
            eprintln!("========================================");
            eprintln!("Public URL: {}", url);
            eprintln!("========================================");
        }
        Err(e) => {
            eprintln!("Warning: Could not get ngrok URL: {}", e);
            eprintln!("Check http://localhost:4040 for the tunnel URL");
        }
    }

    Ok(child)
}

async fn get_ngrok_url() -> Result<String> {
    let client = reqwest::Client::new();
    let resp = client
        .get("http://localhost:4040/api/tunnels")
        .send()
        .await?
        .json::<serde_json::Value>()
        .await?;

    let tunnels = resp["tunnels"]
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("No tunnels found"))?;

    for tunnel in tunnels {
        if let Some(url) = tunnel["public_url"].as_str() {
            // Prefer https
            if url.starts_with("https://") {
                return Ok(url.to_string());
            }
        }
    }

    // Fall back to first tunnel
    tunnels
        .first()
        .and_then(|t| t["public_url"].as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| anyhow::anyhow!("No tunnel URL found"))
}
