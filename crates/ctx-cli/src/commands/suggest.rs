//! Suggest command - find related files

use std::path::{Path, PathBuf};

use anyhow::Result;
use ctx_suggest::{SuggestConfig, SuggestRequest, SuggestionEngine};

pub async fn handle_suggest(file: PathBuf, max: usize, format: &str) -> Result<()> {
    // Canonicalize the file path
    let file = file.canonicalize()?;

    // Find workspace root (look for .git or Cargo.toml)
    let workspace = find_workspace_root(&file)?;

    // Create suggestion engine
    let config = SuggestConfig {
        max_results: max,
        ..Default::default()
    };
    let engine = SuggestionEngine::new(&workspace, config);

    // Get suggestions
    let request = SuggestRequest {
        file: file.to_string_lossy().to_string(),
        pack_name: None,
        max_results: Some(max),
    };

    let response = engine.suggest(&request).await?;

    // Output results
    match format {
        "json" => {
            println!("{}", serde_json::to_string_pretty(&response)?);
        }
        _ => {
            println!("Suggestions for: {}", response.file);
            println!("({} ms)\n", response.elapsed_ms);

            if response.suggestions.is_empty() {
                println!("No suggestions found.");
            } else {
                for (i, suggestion) in response.suggestions.iter().enumerate() {
                    // Make path relative to workspace for readability
                    let display_path = suggestion
                        .path
                        .strip_prefix(workspace.to_string_lossy().as_ref())
                        .map(|p| p.trim_start_matches('/'))
                        .unwrap_or(&suggestion.path);

                    println!(
                        "{}. {} (score: {:.0}%)",
                        i + 1,
                        display_path,
                        suggestion.score * 100.0
                    );

                    for reason in &suggestion.reasons {
                        println!(
                            "   - {}: {:.0}%",
                            reason.signal,
                            reason.contribution * 100.0
                        );
                    }
                }
            }
        }
    }

    Ok(())
}

/// Find the workspace root by looking for .git or Cargo.toml
fn find_workspace_root(file: &Path) -> Result<PathBuf> {
    let mut current = if file.is_file() {
        file.parent().unwrap_or(file).to_owned()
    } else {
        file.to_path_buf()
    };

    loop {
        // Check for git repo
        if current.join(".git").exists() {
            return Ok(current);
        }
        // Check for Cargo.toml (Rust workspace)
        if current.join("Cargo.toml").exists() {
            return Ok(current);
        }
        // Check for package.json (Node.js project)
        if current.join("package.json").exists() {
            return Ok(current);
        }

        if !current.pop() {
            // No parent, use original file's directory
            return Ok(file.parent().unwrap_or(file).to_owned());
        }
    }
}
