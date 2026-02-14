use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use garde::Validate;
use uuid::Uuid;

use crate::auth::middleware::AuthUser;
use crate::error::{AppError, AppResult};
use crate::models::github::{CreateGitHubLinkRequest, GitHubLink, GitHubLinkStatus};
use crate::services::{authorization_service, github_link_service};
use crate::AppState;

#[utoipa::path(
    post,
    path = "/api/projects/{project_id}/github-link",
    params(("project_id" = Uuid, Path, description = "Project ID")),
    request_body = CreateGitHubLinkRequest,
    responses(
        (status = 201, description = "GitHub link created", body = GitHubLink),
        (status = 400, description = "Validation error"),
        (status = 403, description = "Forbidden"),
        (status = 409, description = "Link already exists")
    ),
    tag = "github-links"
)]
pub async fn create_github_link(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(project_id): Path<Uuid>,
    Json(body): Json<CreateGitHubLinkRequest>,
) -> AppResult<(StatusCode, Json<GitHubLink>)> {
    body.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;
    authorization_service::require_project_admin(&state.pool, auth.user_id, project_id).await?;
    let link = github_link_service::create_github_link(&state.pool, project_id, &body).await?;
    Ok((StatusCode::CREATED, Json(link)))
}

#[utoipa::path(
    get,
    path = "/api/projects/{project_id}/github-link",
    params(("project_id" = Uuid, Path, description = "Project ID")),
    responses(
        (status = 200, description = "GitHub link", body = GitHubLink),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Not found")
    ),
    tag = "github-links"
)]
pub async fn get_github_link(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(project_id): Path<Uuid>,
) -> AppResult<Json<GitHubLink>> {
    authorization_service::authorize_project(&state.pool, auth.user_id, project_id).await?;
    let link = github_link_service::get_github_link(&state.pool, project_id).await?;
    Ok(Json(link))
}

#[utoipa::path(
    delete,
    path = "/api/projects/{project_id}/github-link",
    params(("project_id" = Uuid, Path, description = "Project ID")),
    responses(
        (status = 204, description = "GitHub link deleted"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Not found")
    ),
    tag = "github-links"
)]
pub async fn delete_github_link(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(project_id): Path<Uuid>,
) -> AppResult<StatusCode> {
    authorization_service::require_project_admin(&state.pool, auth.user_id, project_id).await?;
    github_link_service::delete_github_link(&state.pool, project_id).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(
    get,
    path = "/api/projects/{project_id}/github-link/status",
    params(("project_id" = Uuid, Path, description = "Project ID")),
    responses(
        (status = 200, description = "GitHub link status", body = GitHubLinkStatus),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Not found")
    ),
    tag = "github-links"
)]
pub async fn get_github_link_status(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(project_id): Path<Uuid>,
) -> AppResult<Json<GitHubLinkStatus>> {
    authorization_service::authorize_project(&state.pool, auth.user_id, project_id).await?;
    let link = github_link_service::get_github_link(&state.pool, project_id).await?;

    let connected = match &state.github_client {
        Some(client) => client.check_connection().await.unwrap_or(false),
        None => false,
    };

    Ok(Json(GitHubLinkStatus {
        project_id: link.project_id,
        repo_owner: link.repo_owner,
        repo_name: link.repo_name,
        connected,
        last_synced_at: link.last_synced_at,
    }))
}
