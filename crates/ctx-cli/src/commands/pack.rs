use anyhow::Result;
use ctx_config::{ArtifactDefinition, Config, PackDefinition, ProjectConfig};
use ctx_core::{OrderingStrategy, Pack, RenderPolicy};
use ctx_engine::Renderer;
use ctx_sources::{Denylist, SourceHandlerRegistry, SourceOptions};
use ctx_storage::Storage;
use std::path::Path;

use crate::cli::PackCommands;

pub async fn handle(cmd: PackCommands, storage: &Storage, config: &Config) -> Result<()> {
    let denylist = Denylist::new(config.denylist.patterns.clone());
    match cmd {
        PackCommands::Create { name, tokens } => {
            let budget = tokens.unwrap_or(config.budget_tokens);
            create(storage, name, budget).await
        }
        PackCommands::List => list(storage).await,
        PackCommands::Show { pack } => show(storage, pack).await,
        PackCommands::Add {
            pack,
            source,
            priority,
            start,
            end,
            max_files,
            exclude,
            recursive,
        } => {
            add(
                storage, &denylist, pack, source, priority, start, end, max_files, exclude,
                recursive,
            )
            .await
        }
        PackCommands::Remove { pack, artifact_id } => remove(storage, pack, artifact_id).await,
        PackCommands::Preview {
            pack,
            tokens,
            redactions,
            show_payload,
        } => preview(storage, pack, tokens, redactions, show_payload).await,
        PackCommands::Delete { pack, force } => delete(storage, pack, force).await,
        PackCommands::Sync => sync(storage, config, &denylist).await,
        PackCommands::Save { packs, all } => save(storage, packs, all).await,
    }
}

async fn create(storage: &Storage, name: String, tokens: usize) -> Result<()> {
    let policies = RenderPolicy {
        budget_tokens: tokens,
        ordering: OrderingStrategy::PriorityThenTime,
    };

    let pack = Pack::new(name.clone(), policies);
    storage.create_pack(&pack).await?;

    println!("✓ Created pack: {}", name);
    println!("  ID: {}", pack.id);
    println!("  Token budget: {}", tokens);

    Ok(())
}

async fn list(storage: &Storage) -> Result<()> {
    let packs = storage.list_packs().await?;

    if packs.is_empty() {
        println!("No packs found.");
        return Ok(());
    }

    println!("Packs:");
    for pack in packs {
        println!("  {} ({})", pack.name, pack.id);
        println!("    Token budget: {}", pack.policies.budget_tokens);
    }

    Ok(())
}

async fn show(storage: &Storage, pack_name: String) -> Result<()> {
    // Get pack by name or ID
    let pack = storage.get_pack(&pack_name).await?;

    println!("Pack: {}", pack.name);
    println!("  ID: {}", pack.id);
    println!("  Token budget: {}", pack.policies.budget_tokens);
    println!("  Created: {}", pack.created_at);
    println!("  Updated: {}", pack.updated_at);

    let artifacts = storage.get_pack_artifacts(&pack.id).await?;

    if artifacts.is_empty() {
        println!("\nNo artifacts.");
    } else {
        println!("\nArtifacts ({}):", artifacts.len());
        for item in artifacts {
            println!(
                "  [{}] {} (priority: {})",
                item.artifact.id, item.artifact.source_uri, item.priority
            );
            let type_json = serde_json::to_string_pretty(&item.artifact.artifact_type)?;
            println!("    Type: {}", type_json);
        }
    }

    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn add(
    storage: &Storage,
    denylist: &Denylist,
    pack_name: String,
    source: String,
    priority: i64,
    start: Option<usize>,
    end: Option<usize>,
    max_files: Option<usize>,
    exclude: Vec<String>,
    recursive: bool,
) -> Result<()> {
    let registry = SourceHandlerRegistry::new();

    // Get pack
    let pack = storage.get_pack(&pack_name).await?;

    // Parse source into artifact
    let options = SourceOptions {
        range: start.and_then(|s| end.map(|e| (s, e))),
        max_files,
        exclude,
        recursive,
        priority,
    };

    let artifact = registry.parse(&source, options).await?;

    // Check denylist for file artifacts
    if let ctx_core::ArtifactType::File { path } | ctx_core::ArtifactType::FileRange { path, .. } =
        &artifact.artifact_type
    {
        if denylist.is_denied(path) {
            let pattern = denylist
                .matching_pattern(path)
                .unwrap_or_else(|| "unknown".to_string());
            anyhow::bail!(
                "File '{}' is denied by pattern '{}'. This file may contain sensitive information.",
                path,
                pattern
            );
        }
    }

    // Check if artifact is a collection
    let is_collection = matches!(
        artifact.artifact_type,
        ctx_core::ArtifactType::CollectionMdDir { .. }
            | ctx_core::ArtifactType::CollectionGlob { .. }
    );

    if is_collection {
        // Collections don't have content to load immediately
        storage.create_artifact(&artifact).await?;
        storage
            .add_artifact_to_pack(&pack.id, &artifact.id, priority)
            .await?;
    } else {
        // Load artifact content
        let content = registry.load(&artifact).await?;

        // Store artifact with content and add to pack (atomic transaction)
        storage
            .add_artifact_to_pack_with_content(&pack.id, &artifact, &content, priority)
            .await?;
    }

    println!("✓ Added artifact to pack '{}'", pack.name);
    println!("  Artifact ID: {}", artifact.id);
    println!("  Source: {}", artifact.source_uri);
    println!("  Priority: {}", priority);

    Ok(())
}

async fn remove(storage: &Storage, pack_name: String, artifact_id: String) -> Result<()> {
    // Get pack
    let pack = storage.get_pack(&pack_name).await?;

    // Remove artifact from pack
    storage
        .remove_artifact_from_pack(&pack.id, &artifact_id)
        .await?;

    println!(
        "✓ Removed artifact {} from pack '{}'",
        artifact_id, pack.name
    );

    Ok(())
}

async fn preview(
    storage: &Storage,
    pack_name: String,
    show_tokens: bool,
    show_redactions: bool,
    show_payload: bool,
) -> Result<()> {
    let renderer = Renderer::new(storage.clone());
    let pack = storage.get_pack(&pack_name).await?;

    println!("Previewing pack: {} ({})", pack.name, pack.id);

    let result = renderer.render_pack(&pack.id, None).await?;

    println!("render_hash: {}", result.render_hash);
    println!(
        "token_estimate: {} / {}",
        result.token_estimate, result.budget_tokens
    );

    if !result.excluded.is_empty() {
        println!("\nExcluded Artifacts ({}):", result.excluded.len());
        for excluded in &result.excluded {
            println!("  - {} ({})", excluded.source_uri, excluded.reason);
        }
    }

    if !result.warnings.is_empty() {
        println!("\n⚠ Warnings ({}):", result.warnings.len());
        for warning in &result.warnings {
            println!("  - {}", warning);
        }
    }

    if show_redactions && !result.redactions.is_empty() {
        println!("\nRedactions:");
        for summary in &result.redactions {
            println!(
                "  - Artifact {}: {} redactions ({:?})",
                summary.artifact_id, summary.count, summary.types
            );
        }
    }

    if show_tokens {
        println!("\nIncluded Artifacts:");
        for included in &result.included {
            println!(
                "  - {} ({} tokens)",
                included.source_uri, included.token_estimate
            );
        }
    }

    if show_payload {
        println!("\n--- PAYLOAD START ---");
        if let Some(payload) = result.payload {
            println!("{}", payload);
        }
        println!("--- PAYLOAD END ---");
    }

    Ok(())
}

async fn delete(storage: &Storage, pack_name: String, force: bool) -> Result<()> {
    let pack = storage.get_pack(&pack_name).await?;

    if !force {
        print!("Delete pack '{}' and all its artifacts? [y/N] ", pack.name);
        std::io::Write::flush(&mut std::io::stdout())?;

        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;

        if !input.trim().eq_ignore_ascii_case("y") {
            println!("Cancelled.");
            return Ok(());
        }
    }

    storage.delete_pack(&pack.id).await?;
    println!("✓ Deleted pack: {}", pack.name);

    Ok(())
}

async fn sync(storage: &Storage, _config: &Config, denylist: &Denylist) -> Result<()> {
    let (project_root, project_config) = ProjectConfig::find_and_load()?
        .ok_or_else(|| anyhow::anyhow!("No ctx.toml found in current or parent directories"))?;

    let namespace = ProjectConfig::project_namespace(&project_root);
    println!("Syncing packs from ctx.toml (project: {})", namespace);

    let registry = SourceHandlerRegistry::new();
    let mut synced = 0;
    let mut errors = 0;

    for (pack_name, pack_def) in &project_config.packs {
        let full_name = ProjectConfig::namespaced_pack_name(&project_root, pack_name);
        let budget = pack_def
            .budget
            .unwrap_or(project_config.config.default_budget);

        // Check if pack exists, create or update
        let pack = match storage.get_pack(&full_name).await {
            Ok(existing) => {
                // Pack exists - for now just use existing
                // TODO: update budget if changed
                existing
            }
            Err(_) => {
                // Create new pack
                let policies = RenderPolicy {
                    budget_tokens: budget,
                    ordering: OrderingStrategy::PriorityThenTime,
                };
                let new_pack = Pack::new(full_name.clone(), policies);
                storage.create_pack(&new_pack).await?;
                new_pack
            }
        };

        // Clear existing artifacts and re-add from definition
        // (simple approach - could be smarter with diffing)
        let existing_artifacts = storage.get_pack_artifacts(&pack.id).await?;
        for item in existing_artifacts {
            storage
                .remove_artifact_from_pack(&pack.id, &item.artifact.id)
                .await
                .ok(); // Ignore errors
        }

        // Add artifacts from definition
        for artifact_def in &pack_def.artifacts {
            // Resolve relative paths to absolute
            let source = resolve_source(&artifact_def.source, &project_root);

            // Check denylist
            if denylist.is_denied(&source) {
                eprintln!("  Warning: '{}' is denied by denylist, skipping", source);
                continue;
            }

            let options = SourceOptions {
                priority: artifact_def.priority,
                ..Default::default()
            };

            match registry.parse(&source, options).await {
                Ok(artifact) => {
                    let is_collection = matches!(
                        artifact.artifact_type,
                        ctx_core::ArtifactType::CollectionMdDir { .. }
                            | ctx_core::ArtifactType::CollectionGlob { .. }
                    );

                    if is_collection {
                        storage.create_artifact(&artifact).await?;
                        storage
                            .add_artifact_to_pack(&pack.id, &artifact.id, artifact_def.priority)
                            .await?;
                    } else {
                        match registry.load(&artifact).await {
                            Ok(content) => {
                                storage
                                    .add_artifact_to_pack_with_content(
                                        &pack.id,
                                        &artifact,
                                        &content,
                                        artifact_def.priority,
                                    )
                                    .await?;
                            }
                            Err(e) => {
                                eprintln!("  Warning: Could not load '{}': {}", source, e);
                                errors += 1;
                                continue;
                            }
                        }
                    }
                }
                Err(e) => {
                    eprintln!("  Warning: Could not parse '{}': {}", source, e);
                    errors += 1;
                    continue;
                }
            }
        }

        println!("  ✓ {} ({} artifacts)", pack_name, pack_def.artifacts.len());
        synced += 1;
    }

    println!(
        "\nSynced {} pack(s){}",
        synced,
        if errors > 0 {
            format!(" ({} warnings)", errors)
        } else {
            String::new()
        }
    );

    Ok(())
}

async fn save(storage: &Storage, packs: Vec<String>, all: bool) -> Result<()> {
    let current_dir = std::env::current_dir()?;

    // Load existing ctx.toml or create new
    let (project_root, mut project_config) = ProjectConfig::find_and_load()?
        .unwrap_or_else(|| (current_dir.clone(), ProjectConfig::default()));

    let packs_to_save: Vec<String> = if all {
        // Get all packs from DB
        storage
            .list_packs()
            .await?
            .into_iter()
            .map(|p| p.name)
            .collect()
    } else {
        packs
    };

    if packs_to_save.is_empty() {
        println!("No packs to save.");
        return Ok(());
    }

    let mut saved = 0;
    for pack_name in &packs_to_save {
        match export_pack_to_definition(storage, pack_name, &project_root).await {
            Ok((local_name, def)) => {
                project_config.packs.insert(local_name.clone(), def);
                println!("  ✓ {}", local_name);
                saved += 1;
            }
            Err(e) => {
                eprintln!("  Warning: Could not save '{}': {}", pack_name, e);
            }
        }
    }

    project_config.save(&project_root)?;
    println!("\nSaved {} pack(s) to ctx.toml", saved);

    Ok(())
}

/// Export a pack from DB to a PackDefinition
async fn export_pack_to_definition(
    storage: &Storage,
    pack_name: &str,
    project_root: &Path,
) -> Result<(String, PackDefinition)> {
    let pack = storage.get_pack(pack_name).await?;
    let artifacts = storage.get_pack_artifacts(&pack.id).await?;

    let artifact_defs: Vec<ArtifactDefinition> = artifacts
        .into_iter()
        .map(|item| {
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
    if let Some(path) = source_uri.strip_prefix("file:") {
        let path_buf = std::path::PathBuf::from(path);
        if path_buf.is_absolute() {
            if let Ok(rel_path) = path_buf.strip_prefix(project_root) {
                return format!("file:{}", rel_path.display());
            }
        }
        source_uri.to_string()
    } else {
        source_uri.to_string()
    }
}

/// Resolve relative source URIs to absolute paths
fn resolve_source(source_uri: &str, project_root: &Path) -> String {
    if let Some(path) = source_uri.strip_prefix("file:") {
        let path_buf = std::path::PathBuf::from(path);
        if path_buf.is_relative() {
            let abs_path = project_root.join(&path_buf);
            return format!("file:{}", abs_path.display());
        }
        source_uri.to_string()
    } else if let Some(pattern) = source_uri.strip_prefix("glob:") {
        // For globs, prepend project root to make pattern absolute
        if !pattern.starts_with('/') {
            let abs_pattern = project_root.join(pattern);
            return format!("glob:{}", abs_pattern.display());
        }
        source_uri.to_string()
    } else {
        source_uri.to_string()
    }
}
