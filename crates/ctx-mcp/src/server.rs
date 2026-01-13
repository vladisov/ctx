use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{delete, get, post},
    Json, Router,
};
use serde::Deserialize;
use std::sync::Arc;

use ctx_core::{Artifact, ArtifactType, Pack, RenderPolicy};
use tokio::net::TcpListener;
use tower_http::cors::{Any, CorsLayer};
use tracing::info;

use ctx_core::RenderRequest;
use ctx_engine::Renderer;
use ctx_storage::Storage;

use crate::protocol::{JsonRpcRequest, JsonRpcResponse};
use crate::tools::handle_jsonrpc;

// Request body structs for REST API
#[derive(Deserialize)]
struct CreatePackRequest {
    name: String,
    #[serde(default)]
    budget_tokens: Option<usize>,
}

#[derive(Deserialize)]
struct AddArtifactRequest {
    #[serde(flatten)]
    artifact_type: ArtifactType,
    #[serde(default)]
    priority: Option<i64>,
}

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
            .route("/api/packs", get(api_list_packs).post(api_create_pack))
            .route(
                "/api/packs/:name",
                get(api_get_pack).delete(api_delete_pack),
            )
            .route("/api/packs/:name/render", get(api_render_pack))
            .route(
                "/api/packs/:name/artifacts",
                get(api_list_pack_artifacts).post(api_add_artifact),
            )
            .route(
                "/api/packs/:name/artifacts/:artifact_id",
                delete(api_remove_artifact),
            )
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
    Json(handle_jsonrpc(&state.server, req).await)
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

/// POST /api/packs - Create a new pack
async fn api_create_pack(
    State(state): State<AppState>,
    Json(req): Json<CreatePackRequest>,
) -> Response {
    if state.server.read_only {
        return (StatusCode::FORBIDDEN, "Server is in read-only mode").into_response();
    }

    let policies = RenderPolicy {
        budget_tokens: req.budget_tokens.unwrap_or(128000),
        ..Default::default()
    };

    let pack = Pack::new(req.name.clone(), policies);

    match state.server.db.create_pack(&pack).await {
        Ok(()) => (
            StatusCode::CREATED,
            Json(serde_json::json!({
                "id": pack.id,
                "name": pack.name,
                "message": format!("Pack '{}' created", req.name)
            })),
        )
            .into_response(),
        Err(e) => {
            let status = if e.to_string().contains("already exists") {
                StatusCode::CONFLICT
            } else {
                StatusCode::INTERNAL_SERVER_ERROR
            };
            (status, e.to_string()).into_response()
        }
    }
}

/// DELETE /api/packs/:name - Delete a pack
async fn api_delete_pack(State(state): State<AppState>, Path(name): Path<String>) -> Response {
    if state.server.read_only {
        return (StatusCode::FORBIDDEN, "Server is in read-only mode").into_response();
    }

    // First get the pack to get its ID
    let pack = match state.server.db.get_pack(&name).await {
        Ok(p) => p,
        Err(_) => {
            return (StatusCode::NOT_FOUND, format!("Pack '{}' not found", name)).into_response()
        }
    };

    match state.server.db.delete_pack(&pack.id).await {
        Ok(()) => Json(serde_json::json!({
            "message": format!("Pack '{}' deleted", name)
        }))
        .into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

/// POST /api/packs/:name/artifacts - Add artifact to a pack
async fn api_add_artifact(
    State(state): State<AppState>,
    Path(name): Path<String>,
    Json(req): Json<AddArtifactRequest>,
) -> Response {
    if state.server.read_only {
        return (StatusCode::FORBIDDEN, "Server is in read-only mode").into_response();
    }

    // First get the pack to get its ID
    let pack = match state.server.db.get_pack(&name).await {
        Ok(p) => p,
        Err(_) => {
            return (StatusCode::NOT_FOUND, format!("Pack '{}' not found", name)).into_response()
        }
    };

    // Create source_uri from artifact type
    let source_uri = match &req.artifact_type {
        ArtifactType::File { path } => format!("file://{}", path),
        ArtifactType::FileRange { path, start, end } => {
            format!("file://{}#L{}-L{}", path, start, end)
        }
        ArtifactType::Markdown { path } => format!("md://{}", path),
        ArtifactType::CollectionMdDir { path, .. } => format!("mddir://{}", path),
        ArtifactType::CollectionGlob { pattern } => format!("glob://{}", pattern),
        ArtifactType::Text { .. } => "text://inline".to_string(),
        ArtifactType::GitDiff { base, head } => {
            format!("git://diff/{}..{}", base, head.as_deref().unwrap_or("HEAD"))
        }
    };

    let artifact = Artifact::new(req.artifact_type.clone(), source_uri);
    let priority = req.priority.unwrap_or(0);

    // Extract content for Text artifacts, empty string for others
    let content = match &req.artifact_type {
        ArtifactType::Text { content } => content.as_str(),
        _ => "",
    };

    match state
        .server
        .db
        .add_artifact_to_pack_with_content(&pack.id, &artifact, content, priority)
        .await
    {
        Ok(_) => (
            StatusCode::CREATED,
            Json(serde_json::json!({
                "artifact_id": artifact.id,
                "message": format!("Artifact added to pack '{}'", name)
            })),
        )
            .into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

/// GET /api/packs/:name/artifacts - List artifacts in a pack
async fn api_list_pack_artifacts(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Response {
    // First get the pack to get its ID
    let pack = match state.server.db.get_pack(&name).await {
        Ok(p) => p,
        Err(_) => {
            return (StatusCode::NOT_FOUND, format!("Pack '{}' not found", name)).into_response()
        }
    };

    match state.server.db.get_pack_artifacts(&pack.id).await {
        Ok(items) => Json(items).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

/// DELETE /api/packs/:name/artifacts/:artifact_id - Remove artifact from pack
async fn api_remove_artifact(
    State(state): State<AppState>,
    Path((name, artifact_id)): Path<(String, String)>,
) -> Response {
    if state.server.read_only {
        return (StatusCode::FORBIDDEN, "Server is in read-only mode").into_response();
    }

    // First get the pack to get its ID
    let pack = match state.server.db.get_pack(&name).await {
        Ok(p) => p,
        Err(_) => {
            return (StatusCode::NOT_FOUND, format!("Pack '{}' not found", name)).into_response()
        }
    };

    match state
        .server
        .db
        .remove_artifact_from_pack(&pack.id, &artifact_id)
        .await
    {
        Ok(()) => Json(serde_json::json!({
            "message": format!("Artifact '{}' removed from pack '{}'", artifact_id, name)
        }))
        .into_response(),
        Err(e) => {
            let status = if e.to_string().contains("not found") {
                StatusCode::NOT_FOUND
            } else {
                StatusCode::INTERNAL_SERVER_ERROR
            };
            (status, e.to_string()).into_response()
        }
    }
}
