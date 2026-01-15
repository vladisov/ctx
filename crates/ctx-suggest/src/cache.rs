//! Caching utilities for suggestion signals

use std::path::PathBuf;
use std::time::Instant;

use dashmap::DashMap;

/// Cache for git co-change data
pub struct GitCoChangeCache {
    /// Map from file path -> list of (co-changed file, change count)
    pub cochanges: DashMap<PathBuf, Vec<(PathBuf, usize)>>,
    /// When the cache was last built
    pub built_at: Option<Instant>,
    /// Workspace root this cache is for
    pub workspace: Option<PathBuf>,
}

impl GitCoChangeCache {
    pub fn new() -> Self {
        Self {
            cochanges: DashMap::new(),
            built_at: None,
            workspace: None,
        }
    }

    pub fn is_valid(&self, workspace: &PathBuf) -> bool {
        if self.workspace.as_ref() != Some(workspace) {
            return false;
        }
        // Cache is valid for 5 minutes
        self.built_at.is_some_and(|t| t.elapsed().as_secs() < 300)
    }

    pub fn clear(&self) {
        self.cochanges.clear();
    }
}

impl Default for GitCoChangeCache {
    fn default() -> Self {
        Self::new()
    }
}

/// Cache for import graph data
pub struct ImportGraphCache {
    /// Forward edges: file -> files it imports
    pub imports: DashMap<PathBuf, Vec<PathBuf>>,
    /// Reverse edges: file -> files that import it
    pub imported_by: DashMap<PathBuf, Vec<PathBuf>>,
    /// When the cache was last built
    pub built_at: Option<Instant>,
    /// Workspace root this cache is for
    pub workspace: Option<PathBuf>,
}

impl ImportGraphCache {
    pub fn new() -> Self {
        Self {
            imports: DashMap::new(),
            imported_by: DashMap::new(),
            built_at: None,
            workspace: None,
        }
    }

    pub fn is_valid(&self, workspace: &PathBuf) -> bool {
        if self.workspace.as_ref() != Some(workspace) {
            return false;
        }
        // Import cache is valid for 5 minutes
        self.built_at.is_some_and(|t| t.elapsed().as_secs() < 300)
    }

    pub fn clear(&self) {
        self.imports.clear();
        self.imported_by.clear();
    }
}

impl Default for ImportGraphCache {
    fn default() -> Self {
        Self::new()
    }
}
