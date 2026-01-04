use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

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

impl Pack {
    pub fn new(name: String, policies: RenderPolicy) -> Self {
        let now = OffsetDateTime::now_utc();
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name,
            policies,
            created_at: now,
            updated_at: now,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenderPolicy {
    pub budget_tokens: usize,
    pub ordering: OrderingStrategy,
}

impl Default for RenderPolicy {
    fn default() -> Self {
        Self {
            budget_tokens: 128000, // Default to 128k tokens
            ordering: OrderingStrategy::PriorityThenTime,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OrderingStrategy {
    PriorityThenTime, // Default: priority DESC, added_at ASC
}
