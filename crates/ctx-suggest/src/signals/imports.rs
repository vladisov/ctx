//! Import graph signal - finds files based on import relationships

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::RwLock;
use std::time::Instant;

use anyhow::Result;
use async_trait::async_trait;
use ignore::WalkBuilder;
use tracing::debug;

use super::Signal;
use crate::cache::ImportGraphCache;
use crate::parsers;

/// Signal based on import/dependency relationships
pub struct ImportSignal {
    #[allow(dead_code)]
    workspace: PathBuf,
    cache: RwLock<ImportGraphCache>,
}

impl ImportSignal {
    pub fn new(workspace: PathBuf) -> Self {
        Self {
            workspace,
            cache: RwLock::new(ImportGraphCache::new()),
        }
    }

    /// Build the import graph by scanning source files
    async fn build_import_graph(&self, workspace: &Path) -> Result<()> {
        debug!("Building import graph for {:?}", workspace);

        let mut imports_map: HashMap<PathBuf, Vec<PathBuf>> = HashMap::new();
        let mut imported_by_map: HashMap<PathBuf, Vec<PathBuf>> = HashMap::new();

        // Walk directory, respecting .gitignore
        let walker = WalkBuilder::new(workspace)
            .hidden(true)
            .git_ignore(true)
            .build();

        for entry in walker.filter_map(std::result::Result::ok) {
            let path = entry.path();
            if !path.is_file() {
                continue;
            }

            // Check if supported extension
            let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
            if !parsers::is_supported_extension(ext) {
                continue;
            }

            // Parse imports (skip files that fail to parse)
            let Ok(raw_imports) = parsers::parse_imports(path).await else {
                continue;
            };

            // Resolve imports to file paths
            let mut resolved = Vec::new();
            for import in raw_imports {
                if let Some(resolved_path) = resolve_import(workspace, path, ext, &import) {
                    resolved.push(resolved_path);
                }
            }

            // Store forward edges
            let path_buf = path.to_owned();
            imports_map.insert(path_buf.clone(), resolved.clone());

            // Store reverse edges
            for target in resolved {
                imported_by_map
                    .entry(target)
                    .or_default()
                    .push(path_buf.clone());
            }
        }

        // Update cache
        let mut cache = self.cache.write().unwrap();
        cache.imports.clear();
        cache.imported_by.clear();

        for (path, imports) in imports_map {
            cache.imports.insert(path, imports);
        }
        for (path, importers) in imported_by_map {
            cache.imported_by.insert(path, importers);
        }

        cache.built_at = Some(Instant::now());
        cache.workspace = Some(workspace.to_owned());

        debug!("Built import graph with {} files", cache.imports.len());

        Ok(())
    }

    fn ensure_cache(&self, workspace: &Path) -> bool {
        let cache = self.cache.read().unwrap();
        cache.is_valid(&workspace.to_owned())
    }
}

#[async_trait]
impl Signal for ImportSignal {
    fn name(&self) -> &'static str {
        "import"
    }

    async fn score(&self, query: &Path, workspace: &Path) -> Result<Vec<(String, f64)>> {
        if !self.ensure_cache(workspace) {
            self.build_import_graph(workspace).await?;
        }

        let cache = self.cache.read().unwrap();
        let mut scores: HashMap<PathBuf, f64> = HashMap::new();

        // Direct imports: files that query imports (score: 0.8)
        if let Some(imports) = cache.imports.get(query) {
            for imp in imports.value() {
                *scores.entry(imp.clone()).or_default() += 0.8;
            }
        }

        // Reverse imports: files that import query (score: 0.9)
        if let Some(importers) = cache.imported_by.get(query) {
            for imp in importers.value() {
                *scores.entry(imp.clone()).or_default() += 0.9;
            }
        }

        // Transitive (1-hop): files imported by files that import query (score: 0.3)
        if let Some(importers) = cache.imported_by.get(query) {
            for importer in importers.value() {
                if let Some(their_imports) = cache.imports.get(importer) {
                    for transitive in their_imports.value() {
                        if transitive != query {
                            *scores.entry(transitive.clone()).or_default() += 0.3;
                        }
                    }
                }
            }
        }

        // Shared imports: files that import the same things as query (score: 0.2)
        if let Some(query_imports) = cache.imports.get(query) {
            let query_set: std::collections::HashSet<_> = query_imports.value().iter().collect();

            for entry in &cache.imports {
                let other_path = entry.key();
                if other_path == query {
                    continue;
                }

                let other_imports = entry.value();
                let overlap = other_imports
                    .iter()
                    .filter(|i| query_set.contains(i))
                    .count();

                if overlap > 0 {
                    let overlap_score = (overlap as f64 / query_set.len().max(1) as f64) * 0.2;
                    *scores.entry(other_path.clone()).or_default() += overlap_score;
                }
            }
        }

        // Normalize and return
        let max_score = scores.values().copied().fold(0.0_f64, f64::max).max(1.0);
        let results = scores
            .into_iter()
            .filter(|(path, _)| path.exists())
            .map(|(path, score)| (path.to_string_lossy().to_string(), score / max_score))
            .collect();

        Ok(results)
    }

    async fn warm_cache(&self, workspace: &Path) -> Result<()> {
        self.build_import_graph(workspace).await
    }

    fn clear_cache(&self) {
        let mut cache = self.cache.write().unwrap();
        cache.clear();
        cache.built_at = None;
        cache.workspace = None;
    }
}

/// Resolve an import to a file path based on language
fn resolve_import(
    workspace: &Path,
    source_file: &Path,
    ext: &str,
    import: &str,
) -> Option<PathBuf> {
    match ext {
        "rs" => parsers::rust::resolve_import(workspace, source_file, import),
        "ts" | "tsx" | "js" | "jsx" | "mts" | "mjs" => {
            parsers::typescript::resolve_import(workspace, source_file, import)
        }
        "py" => parsers::python::resolve_import(workspace, source_file, import),
        _ => None,
    }
}
