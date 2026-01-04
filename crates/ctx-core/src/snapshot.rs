use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Snapshot {
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    pub render_hash: String,
    pub payload_hash: String,
    #[serde(with = "time::serde::timestamp")]
    pub created_at: OffsetDateTime,
}

impl Snapshot {
    pub fn new(render_hash: String, payload_hash: String, label: Option<String>) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            label,
            render_hash,
            payload_hash,
            created_at: OffsetDateTime::now_utc(),
        }
    }
}

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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exclusion_reason: Option<String>,
}
