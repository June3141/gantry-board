use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema, sqlx::Type)]
#[sqlx(type_name = "TEXT", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    Backlog,
    Todo,
    InProgress,
    InReview,
    Done,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema, sqlx::Type)]
#[sqlx(type_name = "TEXT", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum TaskPriority {
    Low,
    Medium,
    High,
    Urgent,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, sqlx::FromRow)]
pub struct Task {
    pub id: Uuid,
    pub project_id: Uuid,
    pub title: String,
    pub description: Option<String>,
    pub status: TaskStatus,
    pub priority: TaskPriority,
    pub parent_id: Option<Uuid>,
    pub assigned_to: Option<Uuid>,
    pub position: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, ToSchema, garde::Validate)]
pub struct CreateTaskRequest {
    #[garde(skip)]
    pub project_id: Uuid,
    #[garde(length(min = 1, max = 255))]
    pub title: String,
    #[garde(length(max = 10000))]
    pub description: Option<String>,
    #[garde(skip)]
    pub status: Option<TaskStatus>,
    #[garde(skip)]
    pub priority: Option<TaskPriority>,
    #[garde(skip)]
    pub parent_id: Option<Uuid>,
    #[garde(skip)]
    pub assigned_to: Option<Uuid>,
}

#[derive(Debug, Deserialize, ToSchema, garde::Validate)]
pub struct UpdateTaskRequest {
    #[garde(length(min = 1, max = 255))]
    pub title: Option<String>,
    #[garde(length(max = 10000))]
    pub description: Option<String>,
    #[garde(skip)]
    pub status: Option<TaskStatus>,
    #[garde(skip)]
    pub priority: Option<TaskPriority>,
    #[garde(skip)]
    pub parent_id: Option<Uuid>,
    #[garde(skip)]
    pub assigned_to: Option<Uuid>,
    #[garde(skip)]
    pub position: Option<i32>,
}
