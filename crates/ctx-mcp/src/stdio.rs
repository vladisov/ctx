use std::io::{self, BufRead, Write};
use std::sync::Arc;

use ctx_engine::Renderer;
use ctx_storage::Storage;

use crate::protocol::{JsonRpcRequest, JsonRpcResponse};
use crate::server::McpServer;
use crate::tools::handle_jsonrpc;

pub async fn run_stdio(db: Arc<Storage>, read_only: bool) -> anyhow::Result<()> {
    let renderer = Arc::new(Renderer::new((*db).clone()));
    let server = McpServer {
        db,
        renderer,
        read_only,
    };

    let stdin = io::stdin();
    let mut stdout = io::stdout();

    for line in stdin.lock().lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }

        let response = match serde_json::from_str::<JsonRpcRequest>(&line) {
            Ok(req) => handle_jsonrpc(&server, req).await,
            Err(e) => JsonRpcResponse::error(
                serde_json::json!(null),
                -32700,
                &format!("Parse error: {}", e),
            ),
        };

        let output = serde_json::to_string(&response)?;
        writeln!(stdout, "{}", output)?;
        stdout.flush()?;
    }

    Ok(())
}
