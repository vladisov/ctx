//! Source handler trait

use async_trait::async_trait;
use ctx_core::Artifact;

/// Options for parsing source URIs
#[derive(Debug, Clone, Default)]
pub struct SourceOptions {
    pub range: Option<(usize, usize)>,
    pub max_files: Option<usize>,
    pub exclude: Vec<String>,
    pub recursive: bool,
    pub base: Option<String>,
    pub head: Option<String>,
    pub capture: bool,
}

/// Trait for handling different source types
#[async_trait]
pub trait SourceHandler: Send + Sync {
    /// Parse source URI into artifact metadata
    async fn parse(&self, uri: &str, options: SourceOptions) -> anyhow::Result<Artifact>;

    /// Load content from source (called during render)
    async fn load(&self, artifact: &Artifact) -> anyhow::Result<String>;

    /// Expand collection into individual artifacts (for collections only)
    async fn expand(&self, artifact: &Artifact) -> anyhow::Result<Vec<Artifact>>;

    /// Check if this handler can handle the given URI
    fn can_handle(&self, uri: &str) -> bool;
}

// TODO: Implement handlers in M1:
// - FileHandler (file:path, file:path#Lx-Ly)
// - CollectionHandler (md_dir:path, glob:pattern)
// - TextHandler (text:content)
// TODO: Implement in M4:
// - GitHandler (git:diff)
// - CommandHandler (cmd:command)
