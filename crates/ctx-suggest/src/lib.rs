//! Smart context suggestion for ctx
//!
//! Provides intelligent file suggestions based on:
//! - Git co-change history (files frequently modified together)
//! - Import/dependency graphs (files that import each other)

pub mod cache;
pub mod parsers;
pub mod signals;

use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Instant;

use anyhow::Result;
use serde::{Deserialize, Serialize};

use signals::Signal;

/// A suggestion for a related file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Suggestion {
    /// Absolute path to the suggested file
    pub path: String,
    /// Relevance score (0.0 - 1.0, higher = more relevant)
    pub score: f64,
    /// Signals that contributed to this suggestion
    pub reasons: Vec<SuggestionReason>,
}

/// Why a file was suggested
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuggestionReason {
    /// Signal name (e.g., "git_cochange", "import")
    pub signal: String,
    /// Human-readable description
    pub description: String,
    /// Raw score contribution from this signal
    pub contribution: f64,
}

/// Request for suggestions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuggestRequest {
    /// The file to find related files for
    pub file: String,
    /// Optional pack context (to exclude already-included files)
    #[serde(default)]
    pub pack_name: Option<String>,
    /// Maximum results to return
    #[serde(default)]
    pub max_results: Option<usize>,
}

/// Response with suggestions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuggestResponse {
    /// The query file
    pub file: String,
    /// Suggested related files
    pub suggestions: Vec<Suggestion>,
    /// Time taken in milliseconds
    pub elapsed_ms: u64,
}

/// Configuration for the suggestion engine
#[derive(Debug, Clone)]
pub struct SuggestConfig {
    /// Maximum suggestions to return (default: 10)
    pub max_results: usize,
    /// Minimum score threshold (default: 0.1)
    pub min_score: f64,
    /// Git history lookback depth (default: 500)
    pub git_history_depth: usize,
    /// Weight for git co-change signal (default: 0.5)
    pub git_weight: f64,
    /// Weight for import signal (default: 0.5)
    pub import_weight: f64,
}

impl Default for SuggestConfig {
    fn default() -> Self {
        Self {
            max_results: 10,
            min_score: 0.1,
            git_history_depth: 500,
            git_weight: 0.5,
            import_weight: 0.5,
        }
    }
}

/// The main suggestion engine
pub struct SuggestionEngine {
    config: SuggestConfig,
    signals: Vec<Box<dyn Signal>>,
    workspace: PathBuf,
}

impl SuggestionEngine {
    /// Create a new suggestion engine for a workspace
    pub fn new(workspace: impl Into<PathBuf>, config: SuggestConfig) -> Self {
        let workspace = workspace.into();
        let signals: Vec<Box<dyn Signal>> = vec![
            Box::new(signals::git_cochange::GitCoChangeSignal::new(
                workspace.clone(),
                config.git_history_depth,
            )),
            Box::new(signals::imports::ImportSignal::new(workspace.clone())),
        ];

        Self {
            config,
            signals,
            workspace,
        }
    }

    /// Get suggestions for a file
    pub async fn suggest(&self, request: &SuggestRequest) -> Result<SuggestResponse> {
        let start = Instant::now();
        let query_path = PathBuf::from(&request.file);

        // Collect scores from all signals
        let mut combined_scores: HashMap<String, Vec<(String, f64)>> = HashMap::new();

        for signal in &self.signals {
            let scores = signal.score(&query_path, &self.workspace).await?;
            for (path, score) in scores {
                combined_scores
                    .entry(path)
                    .or_default()
                    .push((signal.name().to_string(), score));
            }
        }

        // Build suggestions with weighted scores
        let mut suggestions: Vec<Suggestion> = combined_scores
            .into_iter()
            .filter(|(path, _)| path != &request.file) // Exclude query file
            .map(|(path, signal_scores)| {
                let mut total_score = 0.0;
                let mut reasons = Vec::new();

                for (signal_name, score) in signal_scores {
                    let weight = match signal_name.as_str() {
                        "git_cochange" => self.config.git_weight,
                        "import" => self.config.import_weight,
                        _ => 0.5,
                    };
                    let weighted = score * weight;
                    total_score += weighted;

                    reasons.push(SuggestionReason {
                        signal: signal_name,
                        description: String::new(), // Filled by signals
                        contribution: weighted,
                    });
                }

                // Normalize total score to 0-1 range
                let normalized_score =
                    (total_score / (self.config.git_weight + self.config.import_weight)).min(1.0);

                Suggestion {
                    path,
                    score: normalized_score,
                    reasons,
                }
            })
            .filter(|s| s.score >= self.config.min_score)
            .collect();

        // Sort by score descending
        suggestions.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // Limit results
        let max = request.max_results.unwrap_or(self.config.max_results);
        suggestions.truncate(max);

        Ok(SuggestResponse {
            file: request.file.clone(),
            suggestions,
            elapsed_ms: start.elapsed().as_millis() as u64,
        })
    }

    /// Warm up caches for all signals
    pub async fn warm_cache(&self) -> Result<()> {
        for signal in &self.signals {
            signal.warm_cache(&self.workspace).await?;
        }
        Ok(())
    }
}
