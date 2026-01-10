use anyhow::Result;
use ctx_core::{
    render::{ProcessedArtifact, RenderEngine, RenderResult},
    RenderPolicy,
};
use ctx_security::Redactor;
use ctx_sources::SourceHandlerRegistry;
use ctx_storage::Storage;
use ctx_tokens::TokenEstimator;

pub struct Renderer {
    storage: Storage,
    source_registry: SourceHandlerRegistry,
    token_estimator: TokenEstimator,
    redactor: Redactor,
    render_engine: RenderEngine,
}

impl Renderer {
    pub fn new(storage: Storage) -> Self {
        Self {
            storage,
            source_registry: SourceHandlerRegistry::new(),
            token_estimator: TokenEstimator::new(),
            redactor: Redactor::new(),
            render_engine: RenderEngine::new(),
        }
    }

    pub async fn render_request(&self, req: ctx_core::RenderRequest) -> Result<RenderResult> {
        // Simple sequential rendering and merging for MVP
        let mut combined_result = RenderResult {
            budget_tokens: 0,
            token_estimate: 0,
            included: Vec::new(),
            excluded: Vec::new(),
            redactions: Vec::new(),
            warnings: Vec::new(),
            render_hash: String::new(),
            payload: Some(String::new()),
        };

        for pack_id in req.pack_ids {
            let result = self.render_pack(&pack_id, None).await?;

            // Merge logic
            combined_result.budget_tokens += result.budget_tokens;
            combined_result.token_estimate += result.token_estimate;
            combined_result.included.extend(result.included);
            combined_result.excluded.extend(result.excluded);
            combined_result.redactions.extend(result.redactions);
            combined_result.warnings.extend(result.warnings);

            if let Some(payload) = result.payload {
                if let Some(ref mut existing) = combined_result.payload {
                    if !existing.is_empty() {
                        existing.push_str("\n\n");
                    }
                    existing.push_str(&payload);
                }
            }
        }

        // Re-calculate hash of combined payload
        if let Some(ref payload) = combined_result.payload {
            combined_result.render_hash = blake3::hash(payload.as_bytes()).to_hex().to_string();
        }

        Ok(combined_result)
    }

    pub async fn render_pack(
        &self,
        pack_id: &str,
        policy_overrides: Option<RenderPolicy>,
    ) -> Result<RenderResult> {
        // 1. Get Pack
        let pack = self.storage.get_pack(pack_id).await?;
        let policy = policy_overrides.unwrap_or(pack.policies);

        // 2. Get Artifacts (Already sorted by priority DESC, added_at ASC)
        let pack_artifacts = self.storage.get_pack_artifacts(&pack.id).await?;

        // 3. Expand and Load Artifacts
        let mut processed_artifacts = Vec::new();
        let mut redaction_infos = Vec::new();
        let mut warnings = Vec::new();

        for item in pack_artifacts {
            let artifacts = self.expand_artifact(&item.artifact).await?;

            for artifact in artifacts {
                // Try to load content from disk first, fall back to cached content
                let content = match self.source_registry.load(&artifact).await {
                    Ok(content) => content,
                    Err(e) => {
                        // Try to load from cached blob storage
                        if artifact.content_hash.is_some() {
                            match self.storage.load_artifact_content(&artifact).await {
                                Ok(cached) => {
                                    warnings.push(format!(
                                        "File not found at '{}', using cached content: {}",
                                        artifact.source_uri, e
                                    ));
                                    cached
                                }
                                Err(_) => return Err(e.into()),
                            }
                        } else {
                            return Err(e.into());
                        }
                    }
                };

                // Redact
                let (redacted_content, infos) = self.redactor.redact(&artifact.id, &content);
                redaction_infos.extend(infos);

                // Estimate Tokens
                let token_count = self.token_estimator.estimate(&redacted_content);

                processed_artifacts.push(ProcessedArtifact {
                    artifact,
                    content: redacted_content,
                    token_count,
                    redacted: false,
                });
            }
        }

        // 4. Render
        Ok(self.render_engine.render(
            processed_artifacts,
            policy.budget_tokens,
            redaction_infos,
            warnings,
        )?)
    }

    async fn expand_artifact(
        &self,
        artifact: &ctx_core::Artifact,
    ) -> Result<Vec<ctx_core::Artifact>> {
        use ctx_core::ArtifactType;

        let paths = match &artifact.artifact_type {
            ArtifactType::CollectionMdDir {
                path,
                max_files,
                exclude,
                recursive,
            } => {
                let handler = ctx_sources::collection::CollectionHandler;
                handler
                    .expand_md_dir(path, *max_files, exclude, *recursive)
                    .await?
            }
            ArtifactType::CollectionGlob { pattern } => {
                let handler = ctx_sources::collection::CollectionHandler;
                handler.expand_glob(pattern).await?
            }
            _ => return Ok(vec![artifact.clone()]),
        };

        // Convert paths to artifacts
        let mut expanded = Vec::new();
        for p in paths {
            let uri = format!("file:{}", p);
            let item = self.source_registry.parse(&uri, Default::default()).await?;
            expanded.push(item);
        }
        Ok(expanded)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ctx_core::{Artifact, ArtifactType, Pack, RenderPolicy, RenderRequest};
    use ctx_storage::Storage;

    async fn create_test_storage() -> Storage {
        let test_dir =
            std::env::temp_dir().join(format!("ctx-engine-test-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&test_dir).unwrap();
        let db_path = test_dir.join("test.db");
        Storage::new(Some(db_path)).await.unwrap()
    }

    #[tokio::test]
    async fn test_render_single_pack() {
        let storage = create_test_storage().await;

        // Create a pack with text artifacts
        let pack = Pack::new("test-pack".to_string(), RenderPolicy::default());
        storage.create_pack(&pack).await.unwrap();

        let artifact = Artifact::new(
            ArtifactType::Text {
                content: "Test content".to_string(),
            },
            "text:test".to_string(),
        );
        storage
            .add_artifact_to_pack_with_content(&pack.id, &artifact, "Test content", 0)
            .await
            .unwrap();

        // Render the pack
        let renderer = Renderer::new(storage);
        let result = renderer.render_pack(&pack.id, None).await.unwrap();

        assert!(result.payload.is_some());
        assert!(result.payload.unwrap().contains("Test content"));
        assert!(result.token_estimate > 0);
    }

    #[tokio::test]
    async fn test_render_empty_pack() {
        let storage = create_test_storage().await;

        // Create an empty pack
        let pack = Pack::new("empty-pack".to_string(), RenderPolicy::default());
        storage.create_pack(&pack).await.unwrap();

        // Render the empty pack
        let renderer = Renderer::new(storage);
        let result = renderer.render_pack(&pack.id, None).await.unwrap();

        assert_eq!(result.included.len(), 0);
        assert_eq!(result.token_estimate, 0);
    }

    #[tokio::test]
    async fn test_render_multi_pack() {
        let storage = create_test_storage().await;

        // Create two packs
        let pack1 = Pack::new("pack-1".to_string(), RenderPolicy::default());
        let pack2 = Pack::new("pack-2".to_string(), RenderPolicy::default());
        storage.create_pack(&pack1).await.unwrap();
        storage.create_pack(&pack2).await.unwrap();

        // Add artifacts to each pack
        let artifact1 = Artifact::new(
            ArtifactType::Text {
                content: "Content 1".to_string(),
            },
            "text:1".to_string(),
        );
        let artifact2 = Artifact::new(
            ArtifactType::Text {
                content: "Content 2".to_string(),
            },
            "text:2".to_string(),
        );

        storage
            .add_artifact_to_pack_with_content(&pack1.id, &artifact1, "Content 1", 0)
            .await
            .unwrap();
        storage
            .add_artifact_to_pack_with_content(&pack2.id, &artifact2, "Content 2", 0)
            .await
            .unwrap();

        // Render both packs
        let renderer = Renderer::new(storage);
        let request = RenderRequest {
            pack_ids: vec![pack1.id.clone(), pack2.id.clone()],
        };
        let result = renderer.render_request(request).await.unwrap();

        assert!(result.payload.is_some());
        let payload = result.payload.unwrap();
        assert!(payload.contains("Content 1"));
        assert!(payload.contains("Content 2"));
        assert_eq!(result.included.len(), 2);
    }

    #[tokio::test]
    async fn test_budget_enforcement() {
        let storage = create_test_storage().await;

        // Create a pack with small budget
        let mut policy = RenderPolicy::default();
        policy.budget_tokens = 10; // Very small budget

        let pack = Pack::new("budget-pack".to_string(), policy);
        storage.create_pack(&pack).await.unwrap();

        // Add a large text artifact
        let artifact = Artifact::new(
            ArtifactType::Text {
                content: "This is a very long piece of content that will exceed the token budget"
                    .to_string(),
            },
            "text:long".to_string(),
        );
        storage
            .add_artifact_to_pack_with_content(
                &pack.id,
                &artifact,
                "This is a very long piece of content that will exceed the token budget",
                0,
            )
            .await
            .unwrap();

        // Render - should enforce budget
        let renderer = Renderer::new(storage);
        let result = renderer.render_pack(&pack.id, None).await.unwrap();

        // Should have excluded items due to budget
        assert!(result.excluded.len() > 0 || result.token_estimate <= 10);
    }

    #[tokio::test]
    async fn test_redaction_integration() {
        let storage = create_test_storage().await;

        // Create pack with content containing secrets
        let pack = Pack::new("secret-pack".to_string(), RenderPolicy::default());
        storage.create_pack(&pack).await.unwrap();

        let artifact = Artifact::new(
            ArtifactType::Text {
                content: "My AWS key is AKIAIOSFODNN7EXAMPLE".to_string(),
            },
            "text:secret".to_string(),
        );
        storage
            .add_artifact_to_pack_with_content(
                &pack.id,
                &artifact,
                "My AWS key is AKIAIOSFODNN7EXAMPLE",
                0,
            )
            .await
            .unwrap();

        // Render - should redact secrets
        let renderer = Renderer::new(storage);
        let result = renderer.render_pack(&pack.id, None).await.unwrap();

        assert!(result.redactions.len() > 0);
        assert!(result.payload.is_some());
        let payload = result.payload.unwrap();
        assert!(payload.contains("[REDACTED:AWS_ACCESS_KEY]"));
        assert!(!payload.contains("AKIAIOSFODNN7EXAMPLE"));
    }

    #[tokio::test]
    async fn test_pack_not_found() {
        let storage = create_test_storage().await;
        let renderer = Renderer::new(storage);

        let result = renderer.render_pack("nonexistent-pack", None).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_deterministic_hash() {
        let storage = create_test_storage().await;

        // Create pack with artifact
        let pack = Pack::new("deterministic-pack".to_string(), RenderPolicy::default());
        storage.create_pack(&pack).await.unwrap();

        let artifact = Artifact::new(
            ArtifactType::Text {
                content: "Deterministic content".to_string(),
            },
            "text:det".to_string(),
        );
        storage
            .add_artifact_to_pack_with_content(&pack.id, &artifact, "Deterministic content", 0)
            .await
            .unwrap();

        // Render twice
        let renderer = Renderer::new(storage);
        let result1 = renderer.render_pack(&pack.id, None).await.unwrap();
        let result2 = renderer.render_pack(&pack.id, None).await.unwrap();

        // Hashes should be the same
        assert_eq!(result1.render_hash, result2.render_hash);
    }
}
