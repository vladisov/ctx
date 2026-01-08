use async_trait::async_trait;
use ctx_core::{Artifact, ArtifactMetadata, ArtifactType, Error, Result};

use crate::handler::{SourceHandler, SourceOptions};

pub struct FileHandler;

#[async_trait]
impl SourceHandler for FileHandler {
    async fn parse(&self, uri: &str, options: SourceOptions) -> Result<Artifact> {
        let path = if let Some(stripped) = uri.strip_prefix("file:") {
            stripped
        } else {
            uri
        };

        // Check if path has line range (e.g., file.txt#L10-L20)
        let (relative_path, range) = if let Some((path, range_str)) = path.split_once("#L") {
            let range = parse_line_range(range_str)?;
            (path.to_string(), Some(range))
        } else {
            (path.to_string(), options.range)
        };

        // Convert to absolute path
        let file_path = std::fs::canonicalize(&relative_path)
            .map_err(|e| {
                Error::Other(anyhow::anyhow!(
                    "Failed to resolve absolute path for {}: {}",
                    relative_path,
                    e
                ))
            })?
            .to_string_lossy()
            .to_string();

        // Read file to compute hash and metadata
        let content = tokio::fs::read_to_string(&file_path).await.map_err(|e| {
            Error::Other(anyhow::anyhow!("Failed to read file {}: {}", file_path, e))
        })?;

        let content_hash = blake3::hash(content.as_bytes()).to_hex().to_string();

        let artifact_type = if let Some((start, end)) = range {
            ArtifactType::FileRange {
                path: file_path.clone(),
                start,
                end,
            }
        } else if file_path.ends_with(".md") {
            ArtifactType::Markdown {
                path: file_path.clone(),
            }
        } else {
            ArtifactType::File {
                path: file_path.clone(),
            }
        };

        let metadata = ArtifactMetadata {
            size_bytes: content.len(),
            mime_type: None,
            extra: serde_json::json!({}),
        };

        Ok(Artifact::new(artifact_type, uri.to_string())
            .with_hash(content_hash)
            .with_metadata(metadata))
    }

    async fn load(&self, artifact: &Artifact) -> Result<String> {
        match &artifact.artifact_type {
            ArtifactType::File { path } | ArtifactType::Markdown { path } => {
                tokio::fs::read_to_string(path).await.map_err(|e| {
                    Error::Other(anyhow::anyhow!("Failed to read file {}: {}", path, e))
                })
            }
            ArtifactType::FileRange { path, start, end } => {
                let content = tokio::fs::read_to_string(path).await.map_err(|e| {
                    Error::Other(anyhow::anyhow!("Failed to read file {}: {}", path, e))
                })?;

                let lines: Vec<_> = content.lines().collect();
                if *start >= lines.len() || *end >= lines.len() {
                    return Err(Error::Other(anyhow::anyhow!(
                        "Line range {}-{} out of bounds for file {} ({} lines)",
                        start,
                        end,
                        path,
                        lines.len()
                    )));
                }

                Ok(lines[*start..=*end].join("\n"))
            }
            _ => Err(Error::Other(anyhow::anyhow!(
                "Unsupported artifact type for FileHandler"
            ))),
        }
    }

    fn can_handle(&self, uri: &str) -> bool {
        uri.starts_with("file:") || (!uri.contains(':') && !uri.starts_with("text:"))
    }
}

fn parse_line_range(range_str: &str) -> Result<(usize, usize)> {
    if let Some((start_str, end_str)) = range_str.split_once('-') {
        let start = start_str
            .trim_start_matches('L')
            .parse::<usize>()
            .map_err(|e| Error::InvalidSourceUri(format!("Invalid start line: {}", e)))?;
        let end = end_str
            .trim_start_matches('L')
            .parse::<usize>()
            .map_err(|e| Error::InvalidSourceUri(format!("Invalid end line: {}", e)))?;

        if start > end {
            return Err(Error::InvalidSourceUri(
                "Start line must be <= end line".to_string(),
            ));
        }

        Ok((start.saturating_sub(1), end.saturating_sub(1))) // Convert to 0-indexed
    } else {
        Err(Error::InvalidSourceUri(format!(
            "Invalid line range format: {}",
            range_str
        )))
    }
}
