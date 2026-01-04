use std::sync::Arc;
use anyhow::Result;
use ctx_core::{
    RenderPolicy,
    render::{RenderEngine, ProcessedArtifact, RenderResult},
};
use ctx_storage::Storage;
use ctx_sources::SourceHandlerRegistry;
use ctx_tokens::TokenEstimator;
use ctx_security::Redactor;

pub struct Renderer {
    storage: Storage,
    source_registry: SourceHandlerRegistry,
    token_estimator: Arc<TokenEstimator>,
    redactor: Arc<Redactor>,
    render_engine: RenderEngine,
}

impl Renderer {
    pub fn new(storage: Storage) -> Self {
        Self {
            storage,
            source_registry: SourceHandlerRegistry::new(),
            token_estimator: Arc::new(TokenEstimator::new()),
            redactor: Arc::new(Redactor::new()),
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

    pub async fn render_pack(&self, pack_id: &str, policy_overrides: Option<RenderPolicy>) -> Result<RenderResult> {
        // 1. Get Pack
        let pack = self.storage.get_pack(pack_id).await?;
        let policy = policy_overrides.unwrap_or(pack.policies);

        // 2. Get Artifacts (Already sorted by priority DESC, added_at ASC)
        let pack_artifacts = self.storage.get_pack_artifacts(&pack.id).await?;

        // 3. Expand and Load Artifacts
        let mut processed_artifacts = Vec::new();
        let mut redaction_infos = Vec::new();

        for item in pack_artifacts {
             let artifacts = self.expand_artifact(&item.artifact).await?;

            for artifact in artifacts {
                // Load content
                let content = self.source_registry.load(&artifact).await?;

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
        )?)
    }

    async fn expand_artifact(&self, artifact: &ctx_core::Artifact) -> Result<Vec<ctx_core::Artifact>> {
         use ctx_core::ArtifactType;
         
         match &artifact.artifact_type {
             ArtifactType::CollectionMdDir { path, max_files, exclude, recursive } => {
                let handler = ctx_sources::collection::CollectionHandler;
                let paths = handler.expand_md_dir(path, *max_files, &exclude, *recursive).await?;
                
                let mut expanded = Vec::new();
                for p in paths {
                     // Parse each file as a new artifact
                     let uri = format!("file:{}", p);
                     let item = self.source_registry.parse(&uri, Default::default()).await?;
                     expanded.push(item);
                }
                Ok(expanded)
             }
             ArtifactType::CollectionGlob { pattern } => {
                let handler = ctx_sources::collection::CollectionHandler;
                let paths = handler.expand_glob(pattern).await?;
                
                let mut expanded = Vec::new();
                for p in paths {
                     let uri = format!("file:{}", p);
                     let item = self.source_registry.parse(&uri, Default::default()).await?;
                     expanded.push(item);
                }
                Ok(expanded)
             }
             _ => Ok(vec![artifact.clone()]),
         }
    }
}
