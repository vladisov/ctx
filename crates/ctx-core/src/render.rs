use crate::{Artifact, Result};
use serde::{Deserialize, Serialize};

/// Request to render packs into a payload
#[derive(Debug, Clone)]
pub struct RenderRequest {
    pub pack_ids: Vec<String>,
}

/// Result of rendering
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenderResult {
    pub budget_tokens: usize,
    pub token_estimate: usize,
    pub included: Vec<ArtifactSummary>,
    pub excluded: Vec<ExclusionInfo>,
    pub redactions: Vec<RedactionSummary>,
    pub warnings: Vec<String>,
    pub render_hash: String,
    pub payload: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtifactSummary {
    pub artifact_id: String,
    pub source_uri: String,
    pub token_estimate: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExclusionInfo {
    pub artifact_id: String,
    pub source_uri: String,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedactionSummary {
    pub artifact_id: String,
    pub types: Vec<String>,
    pub count: usize,
}

/// Artifact with its content loaded and processed
#[derive(Debug, Clone)]
pub struct ProcessedArtifact {
    pub artifact: Artifact,
    pub content: String,
    pub token_count: usize,
    pub redacted: bool,
}

impl ProcessedArtifact {
    pub fn summary(&self) -> ArtifactSummary {
        ArtifactSummary {
            artifact_id: self.artifact.id.clone(),
            source_uri: self.artifact.source_uri.clone(),
            token_estimate: self.token_count,
        }
    }

    pub fn exclusion(&self, reason: String) -> ExclusionInfo {
        ExclusionInfo {
            artifact_id: self.artifact.id.clone(),
            source_uri: self.artifact.source_uri.clone(),
            reason,
        }
    }
}

/// Simple deterministic render engine
pub struct RenderEngine;

impl RenderEngine {
    pub fn new() -> Self {
        Self
    }

    /// Render artifacts into a deterministic payload
    ///
    /// CRITICAL: This must be deterministic - same inputs produce same output
    pub fn render(
        &self,
        artifacts: Vec<ProcessedArtifact>,
        budget_tokens: usize,
        redaction_info: Vec<ctx_security::RedactionInfo>,
        warnings: Vec<String>,
    ) -> Result<RenderResult> {
        // Apply budget - keep artifacts until we hit budget (caller pre-sorts by priority)
        let (included, excluded) = self.apply_budget(artifacts, budget_tokens);

        // Concatenate payload in order
        let payload = self.concatenate_payload(&included);

        // Compute hashes for reproducibility
        let render_hash = self.compute_render_hash(&included);

        // Collect redaction summaries
        let redactions = self.summarize_redactions(redaction_info);

        // Calculate totals
        let token_estimate: usize = included.iter().map(|a| a.token_count).sum();

        Ok(RenderResult {
            budget_tokens,
            token_estimate,
            included: included.iter().map(|a| a.summary()).collect(),
            excluded: excluded
                .iter()
                .map(|(a, reason)| a.exclusion(reason.clone()))
                .collect(),
            redactions,
            warnings,
            render_hash,
            payload: Some(payload),
        })
    }

    /// Apply budget: include artifacts until budget is reached
    fn apply_budget(
        &self,
        artifacts: Vec<ProcessedArtifact>,
        budget: usize,
    ) -> (Vec<ProcessedArtifact>, Vec<(ProcessedArtifact, String)>) {
        let mut included = Vec::new();
        let mut excluded = Vec::new();
        let mut total_tokens = 0;

        for artifact in artifacts {
            if total_tokens + artifact.token_count <= budget {
                total_tokens += artifact.token_count;
                included.push(artifact);
            } else {
                excluded.push((artifact, "over_budget".to_string()));
            }
        }

        (included, excluded)
    }

    /// Concatenate artifacts into a single payload
    fn concatenate_payload(&self, artifacts: &[ProcessedArtifact]) -> String {
        let mut payload = String::new();

        for artifact in artifacts {
            // Add header with source info
            payload.push_str(&format!("\n--- {} ---\n", artifact.artifact.source_uri));

            // Add content
            payload.push_str(&artifact.content);
            payload.push('\n');
        }

        payload
    }

    /// Compute deterministic hash of the render
    fn compute_render_hash(&self, artifacts: &[ProcessedArtifact]) -> String {
        let mut hasher = blake3::Hasher::new();

        // Hash artifact IDs and content hashes in order
        for artifact in artifacts {
            hasher.update(artifact.artifact.id.as_bytes());
            if let Some(hash) = &artifact.artifact.content_hash {
                hasher.update(hash.as_bytes());
            }
        }

        hasher.finalize().to_hex().to_string()
    }

    /// Summarize redaction information by artifact
    fn summarize_redactions(
        &self,
        redaction_info: Vec<ctx_security::RedactionInfo>,
    ) -> Vec<RedactionSummary> {
        // Group by artifact_id
        let mut map: std::collections::HashMap<String, (Vec<String>, usize)> =
            std::collections::HashMap::new();

        for info in redaction_info {
            let entry = map
                .entry(info.artifact_id.clone())
                .or_insert((Vec::new(), 0));
            entry.0.push(info.redaction_type);
            entry.1 += info.count;
        }

        map.into_iter()
            .map(|(artifact_id, (types, count))| RedactionSummary {
                artifact_id,
                types,
                count,
            })
            .collect()
    }
}

impl Default for RenderEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ArtifactType;

    fn create_test_artifact(id: &str, content: &str, tokens: usize) -> ProcessedArtifact {
        let mut artifact = Artifact::new(
            ArtifactType::Text {
                content: content.to_string(),
            },
            format!("text:{}", id),
        );
        artifact.id = id.to_string(); // Force stable ID

        ProcessedArtifact {
            artifact,
            content: content.to_string(),
            token_count: tokens,
            redacted: false,
        }
    }

    #[test]
    fn test_budget_enforcement() {
        let engine = RenderEngine::new();

        let artifacts = vec![
            create_test_artifact("a", "content a", 100),
            create_test_artifact("b", "content b", 100),
            create_test_artifact("c", "content c", 100),
        ];

        let (included, excluded) = engine.apply_budget(artifacts, 250);

        assert_eq!(included.len(), 2);
        assert_eq!(excluded.len(), 1);
    }

    #[test]
    fn test_render_determinism() {
        let engine = RenderEngine::new();

        let artifacts1 = vec![
            create_test_artifact("a", "content a", 100),
            create_test_artifact("b", "content b", 100),
        ];

        let artifacts2 = vec![
            create_test_artifact("a", "content a", 100),
            create_test_artifact("b", "content b", 100),
        ];

        let result1 = engine.render(artifacts1, 1000, vec![], vec![]).unwrap();
        let result2 = engine.render(artifacts2, 1000, vec![], vec![]).unwrap();

        // Same inputs should produce same hash
        assert_eq!(result1.render_hash, result2.render_hash);
        assert_eq!(result1.payload, result2.payload);
    }
}
