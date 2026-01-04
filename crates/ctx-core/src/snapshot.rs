//! Snapshot domain model

use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

/// Immutable record of a render
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Snapshot {
    pub id: String,
    pub label: Option<String>,
    pub render_hash: String,
    pub payload_hash: String,
    #[serde(with = "time::serde::timestamp")]
    pub created_at: OffsetDateTime,
}

/// Item in a snapshot (artifact at render time)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotItem {
    pub snapshot_id: String,
    pub artifact_id: String,
    pub content_hash: String,
    pub render_metadata: RenderItemMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenderItemMetadata {
    pub included: bool,
    pub token_estimate: usize,
    pub exclusion_reason: Option<String>,
    pub redactions: Vec<RedactionInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedactionInfo {
    pub redaction_type: String,
    pub count: usize,
}
