use async_trait::async_trait;
use ctx_core::{Artifact, ArtifactMetadata, ArtifactType, Error, Result};
use std::process::Command;

use crate::handler::{SourceHandler, SourceOptions};

pub struct GitHandler;

#[async_trait]
impl SourceHandler for GitHandler {
    async fn parse(&self, uri: &str, _options: SourceOptions) -> Result<Artifact> {
        // Format: git:diff --base=main --head=HEAD
        // Or: git:diff (defaults to HEAD vs working tree)
        if let Some(diff_spec) = uri.strip_prefix("git:diff") {
            let (base, head) = parse_diff_spec(diff_spec.trim());

            let artifact_type = ArtifactType::GitDiff {
                base: base.to_string(),
                head: head.map(|s| s.to_string()),
            };

            let metadata = ArtifactMetadata {
                size_bytes: 0,
                mime_type: Some("text/x-diff".to_string()),
                extra: serde_json::json!({
                    "base": base,
                    "head": head,
                }),
            };

            Ok(Artifact::new(artifact_type, uri.to_string()).with_metadata(metadata))
        } else {
            Err(Error::InvalidSourceUri(format!(
                "Invalid git URI: {}. Expected git:diff [--base=REF] [--head=REF]",
                uri
            )))
        }
    }

    async fn load(&self, artifact: &Artifact) -> Result<String> {
        if let ArtifactType::GitDiff { base, head } = &artifact.artifact_type {
            get_diff(base, head.as_deref())
        } else {
            Err(Error::Other(anyhow::anyhow!(
                "Expected GitDiff artifact type"
            )))
        }
    }

    fn can_handle(&self, uri: &str) -> bool {
        uri.starts_with("git:")
    }
}

/// Parse diff specification from URI
fn parse_diff_spec(spec: &str) -> (&str, Option<&str>) {
    let mut base = "HEAD";
    let mut head = None;

    // Parse --base=REF and --head=REF
    for part in spec.split_whitespace() {
        if let Some(val) = part.strip_prefix("--base=") {
            base = val;
        } else if let Some(val) = part.strip_prefix("--head=") {
            head = Some(val);
        }
    }

    (base, head)
}

/// Get git diff using command line
fn get_diff(base: &str, head: Option<&str>) -> Result<String> {
    let mut cmd = Command::new("git");
    cmd.arg("diff");

    if let Some(h) = head {
        // Diff between two refs
        cmd.arg(format!("{}..{}", base, h));
    } else {
        // Diff between ref and working tree
        cmd.arg(base);
    }

    let output = cmd
        .output()
        .map_err(|e| Error::Other(anyhow::anyhow!("Failed to run git diff: {}", e)))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(Error::Other(anyhow::anyhow!(
            "Git diff failed: {}",
            stderr
        )));
    }

    String::from_utf8(output.stdout)
        .map_err(|e| Error::Other(anyhow::anyhow!("Invalid UTF-8 in git diff: {}", e)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_diff_spec() {
        let (base, head) = parse_diff_spec("");
        assert_eq!(base, "HEAD");
        assert_eq!(head, None);

        let (base, head) = parse_diff_spec("--base=main");
        assert_eq!(base, "main");
        assert_eq!(head, None);

        let (base, head) = parse_diff_spec("--base=main --head=feature-branch");
        assert_eq!(base, "main");
        assert_eq!(head, Some("feature-branch"));
    }

    #[tokio::test]
    async fn test_parse_git_uri() {
        let handler = GitHandler;

        let artifact = handler
            .parse("git:diff --base=main --head=HEAD", SourceOptions::default())
            .await
            .unwrap();

        if let ArtifactType::GitDiff { base, head } = artifact.artifact_type {
            assert_eq!(base, "main");
            assert_eq!(head, Some("HEAD".to_string()));
        } else {
            panic!("Expected GitDiff type, got {:?}", artifact.artifact_type);
        }
    }
}
