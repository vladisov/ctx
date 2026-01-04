//! Artifact domain model

use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

/// A single piece of context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Artifact {
    pub id: String,
    #[serde(rename = "type")]
    pub artifact_type: ArtifactType,
    pub source_uri: String,
    pub content_hash: String,
    pub metadata: ArtifactMetadata,
    pub token_estimate: usize,
    #[serde(with = "time::serde::timestamp")]
    pub created_at: OffsetDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ArtifactType {
    File {
        path: String,
    },
    FileRange {
        path: String,
        start: usize,
        end: usize,
    },
    Markdown {
        path: String,
    },
    CollectionMdDir {
        path: String,
        max_files: Option<usize>,
        exclude: Vec<String>,
        recursive: bool,
    },
    CollectionGlob {
        pattern: String,
    },
    GitDiff {
        base: String,
        head: String,
    },
    CommandOutput {
        command: String,
        cwd: String,
    },
    Text {
        content: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtifactMetadata {
    pub size_bytes: usize,
    pub mime_type: Option<String>,
    #[serde(flatten)]
    pub extra: serde_json::Value,
}

impl Default for ArtifactMetadata {
    fn default() -> Self {
        Self {
            size_bytes: 0,
            mime_type: None,
            extra: serde_json::json!({}),
        }
    }
}
