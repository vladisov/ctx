use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Artifact {
    pub id: String,
    #[serde(flatten)]
    pub artifact_type: ArtifactType,
    pub source_uri: String,
    pub content_hash: Option<String>,
    pub metadata: ArtifactMetadata,
    pub token_estimate: usize,
    #[serde(with = "time::serde::timestamp")]
    pub created_at: OffsetDateTime,
}

impl Artifact {
    pub fn new(artifact_type: ArtifactType, source_uri: String) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            artifact_type,
            source_uri,
            content_hash: None,
            metadata: ArtifactMetadata::default(),
            token_estimate: 0,
            created_at: OffsetDateTime::now_utc(),
        }
    }

    pub fn with_hash(mut self, hash: String) -> Self {
        self.content_hash = Some(hash);
        self
    }

    pub fn with_metadata(mut self, metadata: ArtifactMetadata) -> Self {
        self.metadata = metadata;
        self
    }
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
        #[serde(skip_serializing_if = "Option::is_none")]
        max_files: Option<usize>,
        #[serde(default)]
        exclude: Vec<String>,
        #[serde(default)]
        recursive: bool,
    },
    CollectionGlob {
        pattern: String,
    },
    Text {
        content: String,
    },
    GitDiff {
        base: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        head: Option<String>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ArtifactMetadata {
    pub size_bytes: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
    #[serde(flatten)]
    pub extra: serde_json::Value,
}
