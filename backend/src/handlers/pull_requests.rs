use axum::extract::{Path, State};
use axum::Json;
use uuid::Uuid;

use crate::auth::middleware::AuthUser;
use crate::error::AppResult;
use crate::models::github::GitHubPullRequest;
use crate::services::{authorization_service, github_pr_service};
use crate::AppState;

#[utoipa::path(
    get,
    path = "/api/tasks/{task_id}/pull-requests",
    params(("task_id" = Uuid, Path, description = "Task ID")),
    responses(
        (status = 200, description = "List pull requests", body = Vec<GitHubPullRequest>),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Task not found")
    ),
    tag = "pull-requests"
)]
pub async fn list_pull_requests(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(task_id): Path<Uuid>,
) -> AppResult<Json<Vec<GitHubPullRequest>>> {
    authorization_service::authorize_task(&state.pool, auth.user_id, task_id).await?;
    let prs = github_pr_service::list_prs_for_task(&state.pool, task_id).await?;
    Ok(Json(prs))
}
