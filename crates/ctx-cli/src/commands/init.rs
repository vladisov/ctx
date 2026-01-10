use anyhow::Result;
use ctx_config::{ArtifactDefinition, PackDefinition, ProjectConfig};
use ctx_storage::Storage;
use std::path::Path;

pub async fn handle(storage: &Storage, import: Vec<String>) -> Result<()> {
    let current_dir = std::env::current_dir()?;
    let ctx_toml = current_dir.join("ctx.toml");

    if ctx_toml.exists() {
        anyhow::bail!("ctx.toml already exists in current directory");
    }

    let mut project_config = ProjectConfig::default();

    // Import existing packs if requested
    if !import.is_empty() {
        for pack_name in &import {
            match export_pack_to_definition(storage, pack_name, &current_dir).await {
                Ok((name, def)) => {
                    project_config.packs.insert(name, def);
                    println!("  Imported pack: {}", pack_name);
                }
                Err(e) => {
                    eprintln!("  Warning: Could not import '{}': {}", pack_name, e);
                }
            }
        }
    }

    project_config.save(&current_dir)?;

    println!("✓ Created ctx.toml");
    if project_config.packs.is_empty() {
        println!("  Run 'ctx pack create <name>' to add packs");
    } else {
        println!("  Imported {} pack(s)", project_config.packs.len());
    }

    Ok(())
}

/// Export a pack from DB to a PackDefinition
pub async fn export_pack_to_definition(
    storage: &Storage,
    pack_name: &str,
    project_root: &Path,
) -> Result<(String, PackDefinition)> {
    let pack = storage.get_pack(pack_name).await?;
    let artifacts = storage.get_pack_artifacts(&pack.id).await?;

    let artifact_defs: Vec<ArtifactDefinition> = artifacts
        .into_iter()
        .map(|item| {
            // Convert absolute paths to relative
            let source = make_relative_source(&item.artifact.source_uri, project_root);
            ArtifactDefinition {
                source,
                priority: item.priority,
            }
        })
        .collect();

    // Strip namespace if present
    let local_name = ProjectConfig::strip_namespace(project_root, &pack.name)
        .unwrap_or_else(|| pack.name.clone());

    let definition = PackDefinition {
        budget: Some(pack.policies.budget_tokens),
        artifacts: artifact_defs,
    };

    Ok((local_name, definition))
}

/// Convert absolute paths in source URIs to relative paths
fn make_relative_source(source_uri: &str, project_root: &Path) -> String {
    if let Some(path) = source_uri.strip_prefix("file:")
        && let Ok(abs_path) = std::fs::canonicalize(path)
        && let Ok(rel_path) = abs_path.strip_prefix(project_root)
    {
        return format!("file:{}", rel_path.display());
    }
    source_uri.to_string()
}
