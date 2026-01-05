use axum::{extract::State, routing::post, Json, Router};
use std::sync::Arc;
use tokio::net::TcpListener;
use tracing::info;

use ctx_engine::Renderer;
use ctx_storage::Storage;

use crate::protocol::{JsonRpcRequest, JsonRpcResponse};
use crate::tools::{call_tool, list_tools};

pub struct McpServer {
    pub db: Arc<Storage>,
    pub renderer: Arc<Renderer>,
    pub read_only: bool,
}

#[derive(Clone)]
struct AppState {
    server: Arc<McpServer>,
}

impl McpServer {
    pub async fn serve(
        db: Arc<Storage>,
        host: &str,
        port: u16,
        read_only: bool,
    ) -> anyhow::Result<()> {
        let renderer = Arc::new(Renderer::new((*db).clone()));

        let server = Arc::new(Self {
            db,
            renderer,
            read_only,
        });

        let app_state = AppState { server };

        let app = Router::new()
            .route("/", post(handle_jsonrpc))
            .with_state(app_state);

        let addr = format!("{}:{}", host, port);
        let listener = TcpListener::bind(&addr).await?;

        info!("MCP server listening on {}", addr);

        axum::serve(listener, app).await?;

        Ok(())
    }
}

async fn handle_jsonrpc(
    State(state): State<AppState>,
    Json(req): Json<JsonRpcRequest>,
) -> Json<JsonRpcResponse> {
    match req.method.as_str() {
        "tools/list" => {
            let tools = list_tools(state.server.read_only);
            Json(JsonRpcResponse::success(req.id, tools))
        }
        "tools/call" => match call_tool(&state.server, &req.params).await {
            Ok(result) => Json(JsonRpcResponse::success(req.id, result)),
            Err(e) => Json(JsonRpcResponse::error(req.id, -32000, &e.to_string())),
        },
        _ => Json(JsonRpcResponse::error(req.id, -32601, "Method not found")),
    }
}
