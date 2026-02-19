use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ProjectMessage {
    pub id: Uuid,
    pub project_id: Uuid,
    pub user_id: Uuid,
    pub user_name: String,
    pub content: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, ToSchema, garde::Validate)]
pub struct CreateMessageRequest {
    #[garde(length(min = 1, max = 10000))]
    pub content: String,
}
