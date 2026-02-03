use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, sqlx::FromRow)]
pub struct Project {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub owner_id: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, ToSchema, garde::Validate)]
pub struct CreateProjectRequest {
    #[garde(length(min = 1, max = 100))]
    pub name: String,
    #[garde(length(max = 2000))]
    pub description: Option<String>,
}

#[derive(Debug, Deserialize, ToSchema, garde::Validate)]
pub struct UpdateProjectRequest {
    #[garde(length(min = 1, max = 100))]
    pub name: Option<String>,
    #[garde(length(max = 2000))]
    pub description: Option<String>,
}
