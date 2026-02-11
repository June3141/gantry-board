use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::prelude::FromRow;
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

#[derive(FromRow)]
pub(crate) struct AgentSessionOutputRow {
    pub id: i64,
    pub session_id: String,
    pub sequence: i64,
    pub content: String,
    pub created_at: DateTime<Utc>,
}

impl TryFrom<AgentSessionOutputRow> for AgentSessionOutput {
    type Error = uuid::Error;

    fn try_from(row: AgentSessionOutputRow) -> Result<Self, Self::Error> {
        Ok(AgentSessionOutput {
            id: row.id,
            session_id: row.session_id.parse()?,
            sequence: row.sequence,
            content: row.content,
            created_at: row.created_at,
        })
    }
}
