use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use std::sync::Arc;
use tokio::net::TcpListener;
use tower_http::cors::{Any, CorsLayer};
use tracing::info;

use ctx_core::RenderRequest;
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
            // MCP endpoints
            .route("/", get(handle_info))
            .route("/", post(handle_mcp_post))
            .route("/mcp", get(handle_info))
            .route("/mcp", post(handle_mcp_post))
            .route("/sse", get(handle_info))
            .route("/sse", post(handle_mcp_post))
            // REST API endpoints (for ChatGPT Actions, Gemini, etc.)
            .route("/api/packs", get(api_list_packs))
            .route("/api/packs/:name", get(api_get_pack))
            .route("/api/packs/:name/render", get(api_render_pack))
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

// ============================================================================
// REST API handlers (for ChatGPT Actions, Gemini Extensions, etc.)
// ============================================================================

/// GET /api/packs - List all packs
async fn api_list_packs(State(state): State<AppState>) -> Response {
    match state.server.db.list_packs().await {
        Ok(packs) => Json(packs).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

/// GET /api/packs/:name - Get pack details
async fn api_get_pack(State(state): State<AppState>, Path(name): Path<String>) -> Response {
    match state.server.db.get_pack(&name).await {
        Ok(pack) => Json(pack).into_response(),
        Err(_) => (StatusCode::NOT_FOUND, format!("Pack '{}' not found", name)).into_response(),
    }
}

/// GET /api/packs/:name/render - Render pack content
async fn api_render_pack(State(state): State<AppState>, Path(name): Path<String>) -> Response {
    // First get the pack to verify it exists
    let pack = match state.server.db.get_pack(&name).await {
        Ok(p) => p,
        Err(_) => {
            return (StatusCode::NOT_FOUND, format!("Pack '{}' not found", name)).into_response()
        }
    };

    // Render the pack
    match state
        .server
        .renderer
        .render_request(RenderRequest {
            pack_ids: vec![pack.id],
        })
        .await
    {
        Ok(result) => Json(serde_json::json!({
            "pack": name,
            "token_estimate": result.token_estimate,
            "content": result.payload.unwrap_or_default()
        }))
        .into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}
