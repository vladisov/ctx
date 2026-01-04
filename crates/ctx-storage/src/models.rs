use ctx_core::Artifact;
use time::OffsetDateTime;

/// Represents a pack-artifact association with priority
#[derive(Debug, Clone)]
pub struct PackItem {
    pub pack_id: String,
    pub artifact: Artifact,
    pub priority: i64,
    pub added_at: OffsetDateTime,
}
