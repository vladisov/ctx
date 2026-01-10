use std::io::{self, BufRead, Write};
use std::sync::Arc;

use ctx_engine::Renderer;
use ctx_storage::Storage;

use crate::protocol::{JsonRpcRequest, JsonRpcResponse};
use crate::server::McpServer;
use crate::tools::{call_tool, list_tools};

pub async fn run_stdio(db: Arc<Storage>, read_only: bool) -> anyhow::Result<()> {
    let renderer = Arc::new(Renderer::new((*db).clone()));
    let server = Arc::new(McpServer {
        db,
        renderer,
        read_only,
    });

    let stdin = io::stdin();
    let mut stdout = io::stdout();

    for line in stdin.lock().lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }

        let response = match serde_json::from_str::<JsonRpcRequest>(&line) {
            Ok(req) => handle_request(&server, req).await,
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

async fn handle_request(server: &Arc<McpServer>, req: JsonRpcRequest) -> JsonRpcResponse {
    match req.method.as_str() {
        "initialize" => {
            let result = serde_json::json!({
                "protocolVersion": "2024-11-05",
                "capabilities": { "tools": {} },
                "serverInfo": { "name": "ctx", "version": "0.1.0" }
            });
            JsonRpcResponse::success(req.id, result)
        }
        "initialized" | "notifications/initialized" => {
            JsonRpcResponse::success(req.id, serde_json::json!({}))
        }
        "ping" => JsonRpcResponse::success(req.id, serde_json::json!({})),
        "tools/list" => {
            let tools = list_tools(server.read_only);
            JsonRpcResponse::success(req.id, tools)
        }
        "tools/call" => match call_tool(server, &req.params).await {
            Ok(result) => JsonRpcResponse::success(req.id, result),
            Err(e) => JsonRpcResponse::error(req.id, -32000, &e.to_string()),
        },
        _ => JsonRpcResponse::error(req.id, -32601, &format!("Method not found: {}", req.method)),
    }
}
