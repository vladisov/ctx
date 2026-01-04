use async_trait::async_trait;
use ctx_core::{Artifact, ArtifactMetadata, ArtifactType, Error, Result};
use std::path::Path;

use crate::handler::{SourceHandler, SourceOptions};

pub struct CollectionHandler;

#[async_trait]
impl SourceHandler for CollectionHandler {
    async fn parse(&self, uri: &str, options: SourceOptions) -> Result<Artifact> {
        if let Some(path) = uri.strip_prefix("md_dir:") {
            // Collection of markdown files in directory
            let artifact_type = ArtifactType::CollectionMdDir {
                path: path.to_string(),
                max_files: options.max_files,
                exclude: options.exclude,
                recursive: options.recursive,
            };

            let metadata = ArtifactMetadata {
                size_bytes: 0, // Collections don't have a direct size
                mime_type: Some("application/x-ctx-collection".to_string()),
                extra: serde_json::json!({}),
            };

            Ok(Artifact::new(artifact_type, uri.to_string()).with_metadata(metadata))
        } else if let Some(pattern) = uri.strip_prefix("glob:") {
            // Glob pattern collection
            let artifact_type = ArtifactType::CollectionGlob {
                pattern: pattern.to_string(),
            };

            let metadata = ArtifactMetadata {
                size_bytes: 0,
                mime_type: Some("application/x-ctx-collection".to_string()),
                extra: serde_json::json!({}),
            };

            Ok(Artifact::new(artifact_type, uri.to_string()).with_metadata(metadata))
        } else {
            Err(Error::InvalidSourceUri(format!(
                "Invalid collection URI: {}",
                uri
            )))
        }
    }

    async fn load(&self, _artifact: &Artifact) -> Result<String> {
        // Collections are expanded during rendering, not loaded directly
        Err(Error::Other(anyhow::anyhow!(
            "Collections must be expanded before loading"
        )))
    }

    fn can_handle(&self, uri: &str) -> bool {
        uri.starts_with("md_dir:") || uri.starts_with("glob:")
    }
}

impl CollectionHandler {
    /// Expand md_dir into individual file artifacts
    pub async fn expand_md_dir(
        &self,
        path: &str,
        max_files: Option<usize>,
        exclude: &[String],
        recursive: bool,
    ) -> Result<Vec<String>> {
        let mut files = Vec::new();
        let path = Path::new(path);

        if !path.exists() {
            return Err(Error::Other(anyhow::anyhow!(
                "Directory does not exist: {}",
                path.display()
            )));
        }

        if recursive {
            // Use walkdir for recursive scanning
            for entry in walkdir::WalkDir::new(path)
                .into_iter()
                .filter_entry(|e| !is_excluded(e.path(), exclude))
            {
                let entry = entry.map_err(|e| Error::Other(e.into()))?;
                if entry.file_type().is_file() && is_markdown(entry.path()) {
                    files.push(entry.path().display().to_string());
                }
            }
        } else {
            // Non-recursive: only immediate children
            let mut dir_entries = tokio::fs::read_dir(path).await?;
            while let Some(entry) = dir_entries.next_entry().await? {
                if entry.file_type().await?.is_file() {
                    let path = entry.path();
                    if is_markdown(&path) && !is_excluded(&path, exclude) {
                        files.push(path.display().to_string());
                    }
                }
            }
        }

        // Sort for determinism
        files.sort();

        // Apply max_files limit
        if let Some(max) = max_files {
            files.truncate(max);
        }

        Ok(files)
    }

    /// Expand glob pattern into individual file artifacts
    pub async fn expand_glob(&self, pattern: &str) -> Result<Vec<String>> {
        let mut files = Vec::new();

        for entry in glob::glob(pattern).map_err(|e| Error::Other(e.into()))? {
            let path = entry.map_err(|e| Error::Other(e.into()))?;
            if path.is_file() {
                files.push(path.display().to_string());
            }
        }

        // Sort for determinism
        files.sort();

        Ok(files)
    }
}

fn is_markdown(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| e.eq_ignore_ascii_case("md") || e.eq_ignore_ascii_case("markdown"))
        .unwrap_or(false)
}

fn is_excluded(path: &Path, exclude: &[String]) -> bool {
    let path_str = path.display().to_string();
    exclude.iter().any(|pattern| path_str.contains(pattern))
}
