use ctx_core::Artifact;
use serde::Serialize;
use time::OffsetDateTime;

/// Represents a pack-artifact association with priority
#[derive(Debug, Clone, Serialize)]
pub struct PackItem {
    pub pack_id: String,
    pub artifact: Artifact,
    pub priority: i64,
    #[serde(with = "time::serde::timestamp")]
    pub added_at: OffsetDateTime,
}
