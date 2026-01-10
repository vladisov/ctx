use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Pack not found: {0}")]
    PackNotFound(String),

    #[error("Artifact not found: {0}")]
    ArtifactNotFound(String),

    #[error("Pack already exists: {0}")]
    PackAlreadyExists(String),

    #[error("Invalid source URI: {0}")]
    InvalidSourceUri(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Database error: {0}")]
    Database(String),

    #[error("Other error: {0}")]
    Other(#[from] anyhow::Error),
}

pub type Result<T> = std::result::Result<T, Error>;
