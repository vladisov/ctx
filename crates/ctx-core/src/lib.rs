pub mod artifact;
pub mod error;
pub mod pack;
pub mod snapshot;

pub use artifact::{Artifact, ArtifactMetadata, ArtifactType};
pub use error::{Error, Result};
pub use pack::{OrderingStrategy, Pack, RenderPolicy};
pub use snapshot::{RenderItemMetadata, Snapshot, SnapshotItem};

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

    #[test]
    fn test_snapshot_creation() {
        let snapshot = Snapshot::new(
            "render-hash-123".to_string(),
            "payload-hash-456".to_string(),
            Some("v1.0".to_string()),
        );

        assert!(!snapshot.id.is_empty());
        assert_eq!(snapshot.label, Some("v1.0".to_string()));
        assert_eq!(snapshot.render_hash, "render-hash-123");
    }
}
