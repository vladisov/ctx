use anyhow::Result;
use ctx_config::Config;
use ctx_core::{OrderingStrategy, Pack, RenderPolicy, Snapshot};
use ctx_engine::Renderer;
use ctx_sources::{Denylist, SourceHandlerRegistry, SourceOptions};
use ctx_storage::Storage;

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
        PackCommands::Snapshot { pack, label } => snapshot(storage, pack, label).await,
        PackCommands::Delete { pack, force } => delete(storage, pack, force).await,
        PackCommands::Snapshots { pack } => snapshots(storage, pack).await,
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

async fn snapshot(storage: &Storage, pack_name: String, label: Option<String>) -> Result<()> {
    let renderer = Renderer::new(storage.clone());
    let pack = storage.get_pack(&pack_name).await?;

    println!("Creating snapshot for pack: {}...", pack.name);

    let result = renderer.render_pack(&pack.id, None).await?;
    let payload = result.payload.unwrap_or_default();
    let payload_hash = blake3::hash(payload.as_bytes()).to_hex().to_string();

    let snapshot = Snapshot::new(result.render_hash.clone(), payload_hash, label);

    storage.create_snapshot(&snapshot).await?;

    println!("✓ Snapshot created: {}", snapshot.id);
    println!("  Render Hash: {}", snapshot.render_hash);

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

async fn snapshots(storage: &Storage, pack_name: String) -> Result<()> {
    let pack = storage.get_pack(&pack_name).await?;
    let renderer = Renderer::new(storage.clone());

    // Get current render hash to find matching snapshots
    let result = renderer.render_pack(&pack.id, None).await?;
    let current_hash = &result.render_hash;

    let all_snapshots = storage.list_snapshots(Some(current_hash)).await?;

    if all_snapshots.is_empty() {
        println!(
            "No snapshots for pack '{}' (current render_hash: {})",
            pack.name,
            &current_hash[..12]
        );
        return Ok(());
    }

    println!(
        "Snapshots for pack '{}' ({} total):",
        pack.name,
        all_snapshots.len()
    );
    for snap in all_snapshots {
        let label = snap.label.unwrap_or_else(|| "(no label)".to_string());
        println!("  {} - {} ({})", &snap.id[..8], label, snap.created_at);
    }

    Ok(())
}
