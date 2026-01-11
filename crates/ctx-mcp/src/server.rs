use axum::{
    extract::State,
    routing::{get, post},
    Json, Router,
};
use std::sync::Arc;
use tokio::net::TcpListener;
use tower_http::cors::{Any, CorsLayer};
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

        // Add CORS layer to allow connections from any origin
        let cors = CorsLayer::new()
            .allow_origin(Any)
            .allow_methods(Any)
            .allow_headers(Any);

        let app = Router::new()
            .route("/", get(handle_info))
            .route("/", post(handle_mcp_post))
            .route("/mcp", get(handle_info))
            .route("/mcp", post(handle_mcp_post))
            .route("/sse", get(handle_info))
            .route("/sse", post(handle_mcp_post))
            .layer(cors)
            .with_state(app_state);

        let addr = format!("{}:{}", host, port);
        let listener = TcpListener::bind(&addr).await?;

        info!("MCP server listening on {}", addr);

        axum::serve(listener, app).await?;

        Ok(())
    }
}

/// GET handler for server info/health check
async fn handle_info() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "name": "ctx",
        "version": env!("CARGO_PKG_VERSION"),
        "protocol": "mcp",
        "protocolVersion": "2025-03-26"
    }))
}

/// POST /mcp - Handle JSON-RPC messages (stateless mode)
async fn handle_mcp_post(
    State(state): State<AppState>,
    Json(req): Json<JsonRpcRequest>,
) -> Json<JsonRpcResponse> {
    Json(process_jsonrpc(&state, req).await)
}

async fn process_jsonrpc(state: &AppState, req: JsonRpcRequest) -> JsonRpcResponse {
    match req.method.as_str() {
        "initialize" => {
            // MCP protocol initialization
            let result = serde_json::json!({
                "protocolVersion": "2025-03-26",
                "capabilities": {
                    "tools": {
                        "listChanged": false
                    }
                },
                "serverInfo": {
                    "name": "ctx",
                    "version": env!("CARGO_PKG_VERSION")
                }
            });
            JsonRpcResponse::success(req.id, result)
        }
        "initialized" | "notifications/initialized" => {
            // Notification - no response needed, but return empty success
            JsonRpcResponse::success(req.id, serde_json::json!({}))
        }
        "ping" => {
            // Health check
            JsonRpcResponse::success(req.id, serde_json::json!({}))
        }
        "tools/list" => {
            let tools = list_tools(state.server.read_only);
            JsonRpcResponse::success(req.id, tools)
        }
        "tools/call" => match call_tool(&state.server, &req.params).await {
            Ok(result) => JsonRpcResponse::success(req.id, result),
            Err(e) => JsonRpcResponse::error(req.id, -32000, &e.to_string()),
        },
        _ => JsonRpcResponse::error(req.id, -32601, "Method not found"),
    }
}
