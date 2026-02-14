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

/// A GitHub issue as returned from the API.
#[derive(Debug, Clone)]
pub struct GitHubIssue {
    pub number: u64,
    pub id: u64,
    pub title: String,
    pub body: Option<String>,
    pub state: String,
    pub labels: Vec<String>,
    pub pull_request: bool,
    pub updated_at: DateTime<Utc>,
}

/// Request to create a GitHub issue.
#[derive(Debug)]
pub struct CreateIssueRequest {
    pub title: String,
    pub body: Option<String>,
    pub labels: Vec<String>,
}

/// Request to update a GitHub issue.
#[derive(Debug)]
pub struct UpdateIssueRequest {
    pub title: Option<String>,
    pub body: Option<String>,
    pub state: Option<String>,
    pub labels: Option<Vec<String>>,
}

/// Mapping between a local task and a GitHub issue.
#[derive(Debug, Clone)]
pub struct GitHubIssueMapping {
    pub id: Uuid,
    pub task_id: Uuid,
    pub github_link_id: Uuid,
    pub github_issue_number: i64,
    pub github_issue_id: Option<i64>,
    pub last_local_update: Option<DateTime<Utc>>,
    pub last_remote_update: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

/// Result of a sync operation for a single project.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SyncResult {
    pub project_id: Uuid,
    pub pushed: u32,
    pub pulled: u32,
}

/// Pull request state.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum PrState {
    Open,
    Closed,
}

/// A GitHub pull request linked to a task (DB / API response type).
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct GitHubPullRequest {
    pub id: Uuid,
    pub github_link_id: Uuid,
    pub task_id: Uuid,
    pub pr_number: i64,
    pub title: String,
    pub url: String,
    pub state: PrState,
    pub is_merged: bool,
    pub author: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Intermediate type representing a PR discovered from GitHub timeline events.
#[derive(Debug, Clone)]
pub struct LinkedPr {
    pub pr_number: u64,
    pub title: String,
    pub url: String,
    pub state: String,
    pub is_merged: bool,
    pub author: Option<String>,
}
