pub mod artifact;
pub mod error;
pub mod pack;
pub mod render;

pub use artifact::{Artifact, ArtifactMetadata, ArtifactType};
pub use error::{Error, Result};
pub use pack::{OrderingStrategy, Pack, RenderPolicy};
pub use render::{
    ArtifactSummary, ExclusionInfo, ProcessedArtifact, RedactionSummary, RenderEngine,
    RenderRequest, RenderResult,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pack_creation() {
        let policy = RenderPolicy::default();
        let pack = Pack::new("test-pack".to_string(), policy);

        assert_eq!(pack.name, "test-pack");
        assert_eq!(pack.policies.budget_tokens, 128000);
    }

    #[test]
    fn test_artifact_creation() {
        let artifact = Artifact::new(
            ArtifactType::File {
                path: "/test/file.txt".to_string(),
            },
            "file:/test/file.txt".to_string(),
        );

        assert!(!artifact.id.is_empty());
        assert_eq!(artifact.source_uri, "file:/test/file.txt");
    }
}
