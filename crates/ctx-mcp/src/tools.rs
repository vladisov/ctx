use crate::server::McpServer;
use ctx_core::{OrderingStrategy, Pack, RenderPolicy, RenderRequest};
use ctx_sources::{SourceHandlerRegistry, SourceOptions};
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

            format!(
                "Snapshot created: {}\nRender hash: {}",
                snapshot.id, snapshot.render_hash
            )
        }
        "ctx_packs_create" => {
            if server.read_only {
                anyhow::bail!("Server is in read-only mode");
            }
            let name = args["name"]
                .as_str()
                .ok_or_else(|| anyhow::anyhow!("Missing name parameter"))?;
            let budget = args["budget"].as_u64().unwrap_or(128000) as usize;

            let pack = Pack::new(
                name.to_string(),
                RenderPolicy {
                    budget_tokens: budget,
                    ordering: OrderingStrategy::PriorityThenTime,
                },
            );
            server.db.create_pack(&pack).await?;

            format!(
                "Created pack '{}' with {} token budget (id: {})",
                name, budget, pack.id
            )
        }
        "ctx_packs_add_artifact" => {
            if server.read_only {
                anyhow::bail!("Server is in read-only mode");
            }
            let pack_name = args["pack"]
                .as_str()
                .ok_or_else(|| anyhow::anyhow!("Missing pack parameter"))?;
            let source = args["source"]
                .as_str()
                .ok_or_else(|| anyhow::anyhow!("Missing source parameter"))?;
            let priority = args["priority"].as_i64().unwrap_or(0);

            let pack = server.db.get_pack(pack_name).await?;
            let registry = SourceHandlerRegistry::new();
            let options = SourceOptions {
                priority,
                ..Default::default()
            };

            let artifact = registry.parse(source, options).await?;
            let is_collection = matches!(
                artifact.artifact_type,
                ctx_core::ArtifactType::CollectionMdDir { .. }
                    | ctx_core::ArtifactType::CollectionGlob { .. }
            );

            if is_collection {
                server.db.create_artifact(&artifact).await?;
                server
                    .db
                    .add_artifact_to_pack(&pack.id, &artifact.id, priority)
                    .await?;
            } else {
                let content = registry.load(&artifact).await?;
                server
                    .db
                    .add_artifact_to_pack_with_content(&pack.id, &artifact, &content, priority)
                    .await?;
            }

            format!(
                "Added '{}' to pack '{}' (artifact id: {})",
                source, pack.name, artifact.id
            )
        }
        "ctx_packs_delete" => {
            if server.read_only {
                anyhow::bail!("Server is in read-only mode");
            }
            let pack_name = args["pack"]
                .as_str()
                .ok_or_else(|| anyhow::anyhow!("Missing pack parameter"))?;

            let pack = server.db.get_pack(pack_name).await?;
            server.db.delete_pack(&pack.id).await?;

            format!("Deleted pack '{}'", pack.name)
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
    let mut tools = vec![
        tool_schema(
            "ctx_packs_list",
            "List all context packs",
            json!({"type": "object", "properties": {}}),
        ),
        tool_schema(
            "ctx_packs_get",
            "Get pack details including artifacts",
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
            "Preview pack rendering with token counts",
            json!({
                "type": "object",
                "properties": {
                    "packs": {
                        "type": "array",
                        "items": {"type": "string"},
                        "description": "Pack names or IDs to render"
                    },
                    "show_payload": {"type": "boolean", "default": false, "description": "Include rendered content"}
                },
                "required": ["packs"]
            }),
        ),
        tool_schema(
            "ctx_packs_snapshot",
            "Create immutable snapshot of rendered packs",
            json!({
                "type": "object",
                "properties": {
                    "packs": {"type": "array", "items": {"type": "string"}, "description": "Pack names or IDs"},
                    "label": {"type": "string", "description": "Optional label for snapshot"}
                },
                "required": ["packs"]
            }),
        ),
    ];

    if !read_only {
        tools.extend([
            tool_schema(
                "ctx_packs_create",
                "Create a new context pack",
                json!({
                    "type": "object",
                    "properties": {
                        "name": {"type": "string", "description": "Pack name"},
                        "budget": {"type": "integer", "description": "Token budget (default: 128000)"}
                    },
                    "required": ["name"]
                }),
            ),
            tool_schema(
                "ctx_packs_add_artifact",
                "Add artifact to pack. Sources: file:path, glob:pattern, text:content, git:diff",
                json!({
                    "type": "object",
                    "properties": {
                        "pack": {"type": "string", "description": "Pack name or ID"},
                        "source": {"type": "string", "description": "Source URI (file:path, glob:src/**/*.rs, text:content, git:diff --base=main)"},
                        "priority": {"type": "integer", "description": "Priority (higher = included first, default: 0)"}
                    },
                    "required": ["pack", "source"]
                }),
            ),
            tool_schema(
                "ctx_packs_delete",
                "Delete a pack and all its artifacts",
                json!({
                    "type": "object",
                    "properties": {
                        "pack": {"type": "string", "description": "Pack name or ID"}
                    },
                    "required": ["pack"]
                }),
            ),
        ]);
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
