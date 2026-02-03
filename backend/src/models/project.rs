use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, sqlx::FromRow)]
pub struct Project {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// ProjectMember types - used in PR 2 (Project Member API)
// Defined here alongside the migration schema for consistency.

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, sqlx::Type)]
#[sqlx(type_name = "TEXT", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum MemberRole {
    Owner,
    Admin,
    Member,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, sqlx::FromRow)]
pub struct ProjectMember {
    pub project_id: Uuid,
    pub user_id: Uuid,
    pub role: MemberRole,
    pub created_at: DateTime<Utc>,
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

#[derive(Debug, Deserialize, ToSchema, garde::Validate)]
pub struct AddMemberRequest {
    #[garde(skip)]
    pub user_id: Uuid,
    #[garde(skip)]
    pub role: MemberRole,
}

#[derive(Debug, Deserialize, ToSchema, garde::Validate)]
pub struct UpdateMemberRequest {
    #[garde(skip)]
    pub role: MemberRole,
}
