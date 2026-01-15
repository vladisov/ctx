//! Signals for determining file relationships

pub mod git_cochange;
pub mod imports;

use std::path::Path;

use anyhow::Result;
use async_trait::async_trait;

/// A signal that provides relevance scores between files
#[async_trait]
pub trait Signal: Send + Sync {
    /// Name of the signal
    fn name(&self) -> &'static str;

    /// Score files related to the query file
    /// Returns (file_path, score) pairs where score is 0.0-1.0
    async fn score(&self, query: &Path, workspace: &Path) -> Result<Vec<(String, f64)>>;

    /// Initialize/warm up the signal cache for a workspace
    async fn warm_cache(&self, workspace: &Path) -> Result<()>;

    /// Clear cached data
    fn clear_cache(&self);
}
