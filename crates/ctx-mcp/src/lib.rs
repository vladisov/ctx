pub mod protocol;
pub mod server;
pub mod stdio;
pub mod tools;

pub use server::McpServer;
pub use stdio::run_stdio;

#[cfg(test)]
mod tests {
    use super::*;
    use ctx_core::{Artifact, ArtifactType, Pack, RenderPolicy};
    use ctx_storage::Storage;
    use protocol::JsonRpcResponse;
    use std::sync::Arc;
    use tools::{call_tool, list_tools};

    async fn create_test_storage() -> Storage {
        let test_dir = std::env::temp_dir().join(format!("ctx-mcp-test-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&test_dir).unwrap();
        let db_path = test_dir.join("test.db");
        Storage::new(Some(db_path)).await.unwrap()
    }

    #[test]
    fn test_jsonrpc_response_success() {
        let response =
            JsonRpcResponse::success(serde_json::json!(1), serde_json::json!({"status": "ok"}));

        assert_eq!(response.jsonrpc, "2.0");
        assert_eq!(response.id, serde_json::json!(1));
        assert!(response.result.is_some());
        assert!(response.error.is_none());
    }

    #[test]
    fn test_jsonrpc_response_error() {
        let response = JsonRpcResponse::error(serde_json::json!(1), -32601, "Method not found");

        assert_eq!(response.jsonrpc, "2.0");
        assert_eq!(response.id, serde_json::json!(1));
        assert!(response.result.is_none());
        assert!(response.error.is_some());
        assert_eq!(response.error.unwrap().code, -32601);
    }

    #[tokio::test]
    async fn test_list_tools() {
        // List tools in read-only mode
        let tools_json = list_tools(true);
        assert!(tools_json.is_object());
        assert!(tools_json["tools"].is_array());

        // Should have at least the basic tools
        let tools = tools_json["tools"].as_array().unwrap();
        assert!(tools.len() > 0);

        // Check for expected tool names
        let tool_names: Vec<&str> = tools.iter().filter_map(|t| t["name"].as_str()).collect();
        assert!(tool_names.contains(&"ctx_packs_list"));
        assert!(tool_names.contains(&"ctx_packs_get"));
        assert!(tool_names.contains(&"ctx_packs_preview"));
    }

    #[tokio::test]
    async fn test_call_tool_list_packs() {
        let storage = Arc::new(create_test_storage().await);
        let renderer = Arc::new(ctx_engine::Renderer::new((*storage).clone()));
        let server = Arc::new(McpServer {
            db: storage.clone(),
            renderer,
            read_only: true,
        });

        // Create a test pack
        let pack = Pack::new("test-pack".to_string(), RenderPolicy::default());
        storage.create_pack(&pack).await.unwrap();

        // Call list_packs tool
        let params = serde_json::json!({
            "name": "ctx_packs_list",
            "arguments": {}
        });

        let result = call_tool(&server, &params).await.unwrap();
        assert!(result.is_array());

        let packs = result.as_array().unwrap();
        assert_eq!(packs.len(), 1);
    }

    #[tokio::test]
    async fn test_call_tool_get_pack() {
        let storage = Arc::new(create_test_storage().await);
        let renderer = Arc::new(ctx_engine::Renderer::new((*storage).clone()));
        let server = Arc::new(McpServer {
            db: storage.clone(),
            renderer,
            read_only: true,
        });

        // Create a test pack
        let pack = Pack::new("my-pack".to_string(), RenderPolicy::default());
        storage.create_pack(&pack).await.unwrap();

        // Call get_pack tool
        let params = serde_json::json!({
            "name": "ctx_packs_get",
            "arguments": {
                "pack": "my-pack"
            }
        });

        let result = call_tool(&server, &params).await.unwrap();
        assert!(result.is_object());
        assert_eq!(result["name"], "my-pack");
    }

    #[tokio::test]
    async fn test_call_tool_pack_not_found() {
        let storage = Arc::new(create_test_storage().await);
        let renderer = Arc::new(ctx_engine::Renderer::new((*storage).clone()));
        let server = Arc::new(McpServer {
            db: storage.clone(),
            renderer,
            read_only: true,
        });

        // Call get_pack with nonexistent pack
        let params = serde_json::json!({
            "name": "ctx_packs_get",
            "arguments": {
                "pack": "nonexistent-pack"
            }
        });

        let result = call_tool(&server, &params).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_call_tool_preview() {
        let storage = Arc::new(create_test_storage().await);
        let renderer = Arc::new(ctx_engine::Renderer::new((*storage).clone()));
        let server = Arc::new(McpServer {
            db: storage.clone(),
            renderer,
            read_only: true,
        });

        // Create a pack with artifact
        let pack = Pack::new("preview-pack".to_string(), RenderPolicy::default());
        storage.create_pack(&pack).await.unwrap();

        let artifact = Artifact::new(
            ArtifactType::Text {
                content: "Test content".to_string(),
            },
            "text:test".to_string(),
        );
        storage
            .add_artifact_to_pack_with_content(&pack.id, &artifact, "Test content", 0)
            .await
            .unwrap();

        // Call preview tool
        let params = serde_json::json!({
            "name": "ctx_packs_preview",
            "arguments": {
                "packs": [pack.id],
                "show_payload": false
            }
        });

        let result = call_tool(&server, &params).await.unwrap();
        assert!(result.is_object());
        assert!(result["render_hash"].is_string());
        assert!(result["token_estimate"].is_number());
    }

    #[tokio::test]
    async fn test_call_tool_unknown() {
        let storage = Arc::new(create_test_storage().await);
        let renderer = Arc::new(ctx_engine::Renderer::new((*storage).clone()));
        let server = Arc::new(McpServer {
            db: storage.clone(),
            renderer,
            read_only: true,
        });

        // Call unknown tool
        let params = serde_json::json!({
            "name": "unknown_tool",
            "arguments": {}
        });

        let result = call_tool(&server, &params).await;
        assert!(result.is_err());
    }
}
