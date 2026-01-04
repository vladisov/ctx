use async_trait::async_trait;
use ctx_core::{Artifact, Error, Result};
use std::sync::Arc;

#[derive(Debug, Clone, Default)]
pub struct SourceOptions {
    pub range: Option<(usize, usize)>,
    pub max_files: Option<usize>,
    pub exclude: Vec<String>,
    pub recursive: bool,
    pub priority: i64,
}

#[async_trait]
pub trait SourceHandler: Send + Sync {
    /// Parse source URI into artifact metadata
    async fn parse(&self, uri: &str, options: SourceOptions) -> Result<Artifact>;

    /// Load content from source (called during render)
    async fn load(&self, artifact: &Artifact) -> Result<String>;

    /// Check if this handler can handle the given URI
    fn can_handle(&self, uri: &str) -> bool;
}

pub struct SourceHandlerRegistry {
    handlers: Vec<Arc<dyn SourceHandler>>,
}

impl SourceHandlerRegistry {
    pub fn new() -> Self {
        let mut registry = Self {
            handlers: Vec::new(),
        };

        // Register built-in handlers
        registry.register(Arc::new(crate::file::FileHandler));
        registry.register(Arc::new(crate::text::TextHandler));
        registry.register(Arc::new(crate::collection::CollectionHandler));

        registry
    }

    pub fn register(&mut self, handler: Arc<dyn SourceHandler>) {
        self.handlers.push(handler);
    }

    pub async fn parse(&self, uri: &str, options: SourceOptions) -> Result<Artifact> {
        for handler in &self.handlers {
            if handler.can_handle(uri) {
                return handler.parse(uri, options).await;
            }
        }

        Err(Error::InvalidSourceUri(format!(
            "No handler found for URI: {}",
            uri
        )))
    }

    pub async fn load(&self, artifact: &Artifact) -> Result<String> {
        for handler in &self.handlers {
            if handler.can_handle(&artifact.source_uri) {
                return handler.load(artifact).await;
            }
        }

        Err(Error::InvalidSourceUri(format!(
            "No handler found for URI: {}",
            artifact.source_uri
        )))
    }
}

impl Default for SourceHandlerRegistry {
    fn default() -> Self {
        Self::new()
    }
}
