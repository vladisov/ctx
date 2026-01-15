//! Git co-change signal - finds files frequently modified together

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::RwLock;
use std::time::Instant;

use anyhow::{Context, Result};
use async_trait::async_trait;
use tracing::debug;

use super::Signal;
use crate::cache::GitCoChangeCache;

/// Signal based on git co-change history
pub struct GitCoChangeSignal {
    #[allow(dead_code)]
    workspace: PathBuf,
    history_depth: usize,
    cache: RwLock<GitCoChangeCache>,
}

impl GitCoChangeSignal {
    pub fn new(workspace: PathBuf, history_depth: usize) -> Self {
        Self {
            workspace,
            history_depth,
            cache: RwLock::new(GitCoChangeCache::new()),
        }
    }

    /// Build the co-change index from git history
    fn build_cochange_index(&self, workspace: &Path) -> Result<()> {
        debug!("Building git co-change index for {:?}", workspace);

        // Run git log to get file changes per commit
        let output = Command::new("git")
            .args([
                "log",
                "--name-only",
                "--format=COMMIT:%H",
                "-n",
                &self.history_depth.to_string(),
            ])
            .current_dir(workspace)
            .output()
            .context("Failed to run git log")?;

        if !output.status.success() {
            anyhow::bail!(
                "git log failed: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let commits = parse_git_log(&stdout);

        // Build co-change counts
        let mut cochange_counts: HashMap<PathBuf, HashMap<PathBuf, usize>> = HashMap::new();

        for files in commits {
            // Skip single-file commits (no co-change) and huge commits (likely merges/refactors)
            if files.len() < 2 || files.len() > 50 {
                continue;
            }

            for i in 0..files.len() {
                for j in 0..files.len() {
                    if i != j {
                        let file_i = workspace.join(&files[i]);
                        let file_j = workspace.join(&files[j]);

                        *cochange_counts
                            .entry(file_i)
                            .or_default()
                            .entry(file_j)
                            .or_default() += 1;
                    }
                }
            }
        }

        // Store in cache, sorted by count
        let mut cache = self.cache.write().unwrap();
        cache.cochanges.clear();

        for (file, cochanges) in cochange_counts {
            let mut sorted: Vec<_> = cochanges.into_iter().collect();
            sorted.sort_by(|a, b| b.1.cmp(&a.1)); // Descending by count
            cache.cochanges.insert(file, sorted);
        }

        cache.built_at = Some(Instant::now());
        cache.workspace = Some(workspace.to_owned());

        debug!("Built co-change index with {} files", cache.cochanges.len());

        Ok(())
    }

    fn ensure_cache(&self, workspace: &Path) -> Result<()> {
        let needs_rebuild = {
            let cache = self.cache.read().unwrap();
            !cache.is_valid(&workspace.to_owned())
        };

        if needs_rebuild {
            self.build_cochange_index(workspace)?;
        }

        Ok(())
    }
}

#[async_trait]
impl Signal for GitCoChangeSignal {
    fn name(&self) -> &'static str {
        "git_cochange"
    }

    async fn score(&self, query: &Path, workspace: &Path) -> Result<Vec<(String, f64)>> {
        self.ensure_cache(workspace)?;

        let cache = self.cache.read().unwrap();

        let cochanges = match cache.cochanges.get(query) {
            Some(c) => c.clone(),
            None => return Ok(vec![]),
        };

        if cochanges.is_empty() {
            return Ok(vec![]);
        }

        // Normalize scores: max co-change count = 1.0
        let max_count = cochanges.first().map_or(1, |(_, c)| *c) as f64;

        let results = cochanges
            .iter()
            .filter(|(path, _)| path.exists())
            .map(|(path, count)| {
                let score = (*count as f64) / max_count;
                (path.to_string_lossy().to_string(), score)
            })
            .collect();

        Ok(results)
    }

    async fn warm_cache(&self, workspace: &Path) -> Result<()> {
        self.build_cochange_index(workspace)
    }

    fn clear_cache(&self) {
        let mut cache = self.cache.write().unwrap();
        cache.clear();
        cache.built_at = None;
        cache.workspace = None;
    }
}

/// Parse git log output into list of files per commit
fn parse_git_log(output: &str) -> Vec<Vec<String>> {
    let mut commits = Vec::new();
    let mut current_files = Vec::new();

    for line in output.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        if line.starts_with("COMMIT:") {
            if !current_files.is_empty() {
                commits.push(std::mem::take(&mut current_files));
            }
        } else {
            // It's a file path
            current_files.push(line.to_string());
        }
    }

    // Don't forget the last commit
    if !current_files.is_empty() {
        commits.push(current_files);
    }

    commits
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_git_log() {
        let log = r#"COMMIT:abc123
src/main.rs
src/lib.rs

COMMIT:def456
src/lib.rs
src/utils.rs
src/config.rs
"#;

        let commits = parse_git_log(log);
        assert_eq!(commits.len(), 2);
        assert_eq!(commits[0], vec!["src/main.rs", "src/lib.rs"]);
        assert_eq!(
            commits[1],
            vec!["src/lib.rs", "src/utils.rs", "src/config.rs"]
        );
    }
}
