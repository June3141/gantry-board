use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, ToSchema, sqlx::Type)]
#[sqlx(type_name = "TEXT", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum AgentType {
    ClaudeCode,
    GeminiCli,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema, sqlx::Type)]
#[sqlx(type_name = "TEXT", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum AgentSessionStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AgentSession {
    pub id: Uuid,
    pub task_id: Uuid,
    pub agent_type: AgentType,
    pub status: AgentSessionStatus,
    pub started_at: Option<DateTime<Utc>>,
    pub finished_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, ToSchema, garde::Validate)]
pub struct CreateAgentSessionRequest {
    #[garde(skip)]
    pub agent_type: AgentType,
}

#[derive(Debug, Deserialize, ToSchema, garde::Validate)]
pub struct UpdateAgentSessionRequest {
    #[garde(skip)]
    pub status: AgentSessionStatus,
}

#[derive(Debug, Deserialize, ToSchema, garde::Validate)]
pub struct StartAgentSessionRequest {
    #[garde(skip)]
    pub agent_type: AgentType,
    #[garde(length(min = 1, max = 10000))]
    pub prompt: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct StartAgentSessionResponse {
    pub session: AgentSession,
}
