use anyhow::Result;
use ctx_core::{OrderingStrategy, Pack, RenderPolicy};
use ctx_sources::{SourceHandlerRegistry, SourceOptions};
use ctx_storage::Storage;

use crate::cli::PackCommands;

pub async fn handle(cmd: PackCommands, storage: &Storage) -> Result<()> {
    match cmd {
        PackCommands::Create { name, tokens } => create(storage, name, tokens).await,
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
                storage, pack, source, priority, start, end, max_files, exclude, recursive,
            )
            .await
        }
        PackCommands::Remove { pack, artifact_id } => remove(storage, pack, artifact_id).await,
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
            println!("  [{}] {} (priority: {})", item.artifact.id, item.artifact.source_uri, item.priority);
            let type_json = serde_json::to_string_pretty(&item.artifact.artifact_type)?;
            println!("    Type: {}", type_json);
        }
    }

    Ok(())
}

async fn add(
    storage: &Storage,
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

    // Load artifact content
    let content = registry.load(&artifact).await?;

    // Store artifact with content and add to pack (atomic transaction)
    storage
        .add_artifact_to_pack_with_content(&pack.id, &artifact, &content, priority)
        .await?;

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

    println!("✓ Removed artifact {} from pack '{}'", artifact_id, pack.name);

    Ok(())
}
