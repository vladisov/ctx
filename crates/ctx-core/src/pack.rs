//! Pack domain model

use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

/// A named bundle of artifacts (context pack)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pack {
    pub id: String,
    pub name: String,
    pub policies: RenderPolicy,
    #[serde(with = "time::serde::timestamp")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::timestamp")]
    pub updated_at: OffsetDateTime,
}

/// Policy for rendering a pack
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenderPolicy {
    pub budget_tokens: usize,
    pub ordering: OrderingStrategy,
    pub redaction: RedactionConfig,
}

impl Default for RenderPolicy {
    fn default() -> Self {
        Self {
            budget_tokens: 24000,
            ordering: OrderingStrategy::PriorityThenTime,
            redaction: RedactionConfig::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OrderingStrategy {
    /// Sort by priority DESC, then added_at ASC
    PriorityThenTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedactionConfig {
    pub enabled: bool,
    pub custom_patterns: Vec<String>,
}

impl Default for RedactionConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            custom_patterns: Vec::new(),
        }
    }
}

/// Represents a pack item (artifact in a pack)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackItem {
    pub pack_id: String,
    pub artifact_id: String,
    pub priority: i32,
    #[serde(with = "time::serde::timestamp")]
    pub added_at: OffsetDateTime,
}
