use async_trait::async_trait;
use ctx_core::{Artifact, ArtifactMetadata, ArtifactType, Error, Result};

use crate::handler::{SourceHandler, SourceOptions};

pub struct TextHandler;

#[async_trait]
impl SourceHandler for TextHandler {
    async fn parse(&self, uri: &str, _options: SourceOptions) -> Result<Artifact> {
        let content = if let Some(text) = uri.strip_prefix("text:") {
            text.to_string()
        } else {
            return Err(Error::InvalidSourceUri(format!(
                "Invalid text URI: {}",
                uri
            )));
        };

        let content_hash = blake3::hash(content.as_bytes()).to_hex().to_string();

        let metadata = ArtifactMetadata {
            size_bytes: content.len(),
            mime_type: Some("text/plain".to_string()),
            extra: serde_json::json!({}),
        };

        Ok(Artifact::new(
            ArtifactType::Text {
                content: content.clone(),
            },
            uri.to_string(),
        )
        .with_hash(content_hash)
        .with_metadata(metadata))
    }

    async fn load(&self, artifact: &Artifact) -> Result<String> {
        match &artifact.artifact_type {
            ArtifactType::Text { content } => Ok(content.clone()),
            _ => Err(Error::Other(anyhow::anyhow!(
                "Unsupported artifact type for TextHandler"
            ))),
        }
    }

    fn can_handle(&self, uri: &str) -> bool {
        uri.starts_with("text:")
    }
}
