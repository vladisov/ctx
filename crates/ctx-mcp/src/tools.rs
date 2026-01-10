use crate::server::McpServer;
use ctx_core::RenderRequest;
use serde_json::json;

pub async fn call_tool(
    server: &McpServer,
    params: &serde_json::Value,
) -> anyhow::Result<serde_json::Value> {
    let tool_name = params["name"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("Missing tool name"))?;
    let args = &params["arguments"];

    let result = match tool_name {
        "ctx_packs_list" => {
            let packs = server.db.list_packs().await?;
            serde_json::to_string_pretty(&packs)?
        }
        "ctx_packs_get" => {
            let pack_name = args["pack"]
                .as_str()
                .ok_or_else(|| anyhow::anyhow!("Missing pack parameter"))?;
            let pack = server.db.get_pack(pack_name).await?;
            serde_json::to_string_pretty(&pack)?
        }
        "ctx_packs_preview" => {
            let pack_ids: Vec<String> = serde_json::from_value(args["packs"].clone())?;
            let show_payload = args["show_payload"].as_bool().unwrap_or(false);

            let mut result = server
                .renderer
                .render_request(RenderRequest { pack_ids })
                .await?;

            if !show_payload {
                result.payload = None;
            }

            serde_json::to_string_pretty(&result)?
        }
        "ctx_packs_snapshot" => {
            let pack_ids: Vec<String> = serde_json::from_value(args["packs"].clone())?;
            let label = args["label"].as_str().map(String::from);

            let result = server
                .renderer
                .render_request(RenderRequest { pack_ids })
                .await?;

            let snapshot = ctx_core::Snapshot::new(
                result.render_hash.clone(),
                blake3::hash(result.payload.clone().unwrap_or_default().as_bytes())
                    .to_hex()
                    .to_string(),
                label,
            );

            server.db.create_snapshot(&snapshot).await?;

            format!("Snapshot created: {}\nRender hash: {}", snapshot.id, snapshot.render_hash)
        }
        _ => anyhow::bail!("Unknown tool: {}", tool_name),
    };

    // MCP spec requires content array with type/text objects
    Ok(json!({
        "content": [
            {"type": "text", "text": result}
        ]
    }))
}

pub fn list_tools(read_only: bool) -> serde_json::Value {
    let tools = vec![
        tool_schema("ctx_packs_list", "List all context packs", json!({"type": "object", "properties": {}})),
        tool_schema(
            "ctx_packs_get",
            "Get pack details",
            json!({
                "type": "object",
                "properties": {
                    "pack": {"type": "string", "description": "Pack name or ID"}
                },
                "required": ["pack"]
            }),
        ),
        tool_schema(
            "ctx_packs_preview",
            "Preview pack rendering",
            json!({
                "type": "object",
                "properties": {
                    "packs": {
                        "type": "array",
                        "items": {"type": "string"},
                        "description": "Pack IDs to render"
                    },
                    "show_payload": {"type": "boolean", "default": false}
                },
                "required": ["packs"]
            }),
        ),
        tool_schema(
            "ctx_packs_snapshot",
            "Create snapshot of rendered packs",
            json!({
                "type": "object",
                "properties": {
                    "packs": {"type": "array", "items": {"type": "string"}},
                    "label": {"type": "string"}
                },
                "required": ["packs"]
            }),
        ),
    ];

    if !read_only {
        // Add write tools in future
    }

    json!({ "tools": tools })
}

fn tool_schema(
    name: &str,
    description: &str,
    input_schema: serde_json::Value,
) -> serde_json::Value {
    json!({
        "name": name,
        "description": description,
        "inputSchema": input_schema
    })
}
