//! Error types for ctx-storage

use thiserror::Error;

pub type Result<T> = std::result::Result<T, StorageError>;

#[derive(Error, Debug)]
pub enum StorageError {
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Blob not found: {0}")]
    BlobNotFound(String),

    #[error(transparent)]
    Other(#[from] anyhow::Error),
}
