//! MCP tool implementations

use serde_json::json;

pub fn list_tools(_read_only: bool) -> serde_json::Value {
    // TODO: Implement in M3
    // Return list of available MCP tools
    json!({ "tools": [] })
}

pub async fn call_tool(
    _tool_name: &str,
    _args: &serde_json::Value,
) -> anyhow::Result<serde_json::Value> {
    // TODO: Implement in M3
    // Handle tool calls:
    // - ctx_packs_list
    // - ctx_packs_get
    // - ctx_packs_preview
    // - ctx_packs_render
    // - ctx_packs_snapshot
    // - ctx_artifacts_get
    todo!("Implement call_tool")
}
