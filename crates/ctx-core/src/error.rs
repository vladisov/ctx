//! Error types for ctx-core

use thiserror::Error;

pub type Result<T> = std::result::Result<T, CoreError>;

#[derive(Error, Debug)]
pub enum CoreError {
    #[error("Pack not found: {0}")]
    PackNotFound(String),

    #[error("Artifact not found: {0}")]
    ArtifactNotFound(String),

    #[error("Invalid artifact type: {0}")]
    InvalidArtifactType(String),

    #[error("Render error: {0}")]
    RenderError(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error(transparent)]
    Other(#[from] anyhow::Error),
}
