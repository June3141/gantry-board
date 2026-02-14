use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct GitHubLink {
    pub id: Uuid,
    pub project_id: Uuid,
    pub repo_owner: String,
    pub repo_name: String,
    pub sync_enabled: bool,
    pub last_synced_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, ToSchema, garde::Validate)]
pub struct CreateGitHubLinkRequest {
    #[garde(length(min = 1, max = 100))]
    pub repo_owner: String,
    #[garde(length(min = 1, max = 100))]
    pub repo_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct GitHubLinkStatus {
    pub project_id: Uuid,
    pub repo_owner: String,
    pub repo_name: String,
    pub connected: bool,
    pub last_synced_at: Option<DateTime<Utc>>,
}
