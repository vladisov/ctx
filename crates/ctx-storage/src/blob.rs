use ctx_core::{Error, Result};
use std::path::PathBuf;
use tokio::fs;

/// Content-addressable blob storage using BLAKE3 hashing
#[derive(Clone)]
pub struct BlobStore {
    root: PathBuf,
}

impl BlobStore {
    pub fn new(root: Option<PathBuf>) -> Self {
        let root = root.unwrap_or_else(|| {
            let dirs = directories::ProjectDirs::from("com", "ctx", "ctx").unwrap();
            let data_dir = dirs.data_dir();
            data_dir.join("blobs")
        });

        Self { root }
    }

    /// Store content and return its hash
    pub async fn store(&self, content: &[u8]) -> Result<String> {
        let hash = blake3::hash(content);
        let hash_hex = hash.to_hex().to_string();

        let path = self.blob_path(&hash_hex);

        // Create parent directory if it doesn't exist
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).await?;
        }

        // Only write if file doesn't already exist (content-addressable deduplication)
        if !path.exists() {
            fs::write(&path, content).await?;
        }

        Ok(hash_hex)
    }

    /// Retrieve content by hash
    pub async fn retrieve(&self, hash: &str) -> Result<Vec<u8>> {
        let path = self.blob_path(hash);

        if !path.exists() {
            return Err(Error::Other(anyhow::anyhow!(
                "Blob not found: {}",
                hash
            )));
        }

        let content = fs::read(&path).await?;

        // Verify hash
        let actual_hash = blake3::hash(&content).to_hex().to_string();
        if actual_hash != hash {
            return Err(Error::Other(anyhow::anyhow!(
                "Blob hash mismatch: expected {}, got {}",
                hash,
                actual_hash
            )));
        }

        Ok(content)
    }

    /// Get the file system path for a given hash
    fn blob_path(&self, hash: &str) -> PathBuf {
        // Shard into prefix directories (first 2 chars)
        let prefix = &hash[..2];
        self.root.join("blake3").join(prefix).join(hash)
    }

    /// Check if a blob exists
    pub async fn exists(&self, hash: &str) -> bool {
        tokio::fs::try_exists(self.blob_path(hash))
            .await
            .unwrap_or(false)
    }
}
