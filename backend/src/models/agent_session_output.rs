use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AgentSessionOutput {
    pub id: i64,
    pub session_id: Uuid,
    pub sequence: i64,
    pub content: String,
    pub created_at: DateTime<Utc>,
}
