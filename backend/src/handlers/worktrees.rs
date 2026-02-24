use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::auth::middleware::AuthUser;
use crate::error::AppResult;
use crate::services::{authorization_service, worktree_service};
use crate::AppState;

#[derive(Debug, Serialize, ToSchema)]
pub struct WorktreeResponse {
    pub name: String,
    pub branch: Option<String>,
    pub is_valid: bool,
}

impl From<worktree_service::WorktreeInfo> for WorktreeResponse {
    fn from(info: worktree_service::WorktreeInfo) -> Self {
        Self {
            name: info.name,
            branch: info.branch,
            is_valid: info.is_valid,
        }
    }
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateWorktreeRequest {
    pub name: String,
}

#[utoipa::path(
    get,
    path = "/api/worktrees",
    responses(
        (status = 200, description = "List worktrees", body = Vec<WorktreeResponse>),
    ),
    tag = "worktrees"
)]
pub async fn list_worktrees(
    State(state): State<AppState>,
    auth: AuthUser,
) -> AppResult<Json<Vec<WorktreeResponse>>> {
    authorization_service::require_any_project_membership(&state.pool, auth.user_id).await?;
    let repo_path = state.config.repo_path();
    let worktrees =
        tokio::task::spawn_blocking(move || worktree_service::list_worktrees(&repo_path))
            .await
            .map_err(|e| crate::error::AppError::Internal(e.to_string()))??;
    Ok(Json(worktrees.into_iter().map(Into::into).collect()))
}

#[utoipa::path(
    post,
    path = "/api/worktrees",
    request_body = CreateWorktreeRequest,
    responses(
        (status = 201, description = "Worktree created", body = WorktreeResponse),
        (status = 400, description = "Invalid worktree name"),
        (status = 409, description = "Worktree already exists")
    ),
    tag = "worktrees"
)]
pub async fn create_worktree(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(body): Json<CreateWorktreeRequest>,
) -> AppResult<(StatusCode, Json<WorktreeResponse>)> {
    authorization_service::require_any_project_membership(&state.pool, auth.user_id).await?;
    let repo_path = state.config.repo_path();
    let name = body.name.trim().to_string();
    if name.is_empty() || name.len() > 100 {
        return Err(crate::error::AppError::Validation(
            "Worktree name must be 1-100 characters".to_string(),
        ));
    }
    let info =
        tokio::task::spawn_blocking(move || worktree_service::create_worktree(&repo_path, &name))
            .await
            .map_err(|e| crate::error::AppError::Internal(e.to_string()))??;
    Ok((StatusCode::CREATED, Json(info.into())))
}

#[utoipa::path(
    get,
    path = "/api/worktrees/{name}",
    params(("name" = String, Path, description = "Worktree name")),
    responses(
        (status = 200, description = "Worktree found", body = WorktreeResponse),
        (status = 404, description = "Worktree not found")
    ),
    tag = "worktrees"
)]
pub async fn get_worktree(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(name): Path<String>,
) -> AppResult<Json<WorktreeResponse>> {
    authorization_service::require_any_project_membership(&state.pool, auth.user_id).await?;
    let repo_path = state.config.repo_path();
    let info =
        tokio::task::spawn_blocking(move || worktree_service::get_worktree(&repo_path, &name))
            .await
            .map_err(|e| crate::error::AppError::Internal(e.to_string()))??;
    Ok(Json(info.into()))
}

#[utoipa::path(
    delete,
    path = "/api/worktrees/{name}",
    params(("name" = String, Path, description = "Worktree name")),
    responses(
        (status = 204, description = "Worktree deleted"),
        (status = 404, description = "Worktree not found")
    ),
    tag = "worktrees"
)]
pub async fn delete_worktree(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(name): Path<String>,
) -> AppResult<StatusCode> {
    authorization_service::require_any_project_membership(&state.pool, auth.user_id).await?;
    let repo_path = state.config.repo_path();
    tokio::task::spawn_blocking(move || worktree_service::delete_worktree(&repo_path, &name))
        .await
        .map_err(|e| crate::error::AppError::Internal(e.to_string()))??;
    Ok(StatusCode::NO_CONTENT)
}
