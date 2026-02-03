use axum::extract::{Path, State};
use axum::Json;
use uuid::Uuid;

use crate::error::AppResult;
use crate::models::project::{CreateProjectRequest, Project, UpdateProjectRequest};
use crate::AppState;

#[utoipa::path(
    get,
    path = "/api/projects",
    responses(
        (status = 200, description = "List all projects", body = Vec<Project>)
    ),
    tag = "projects"
)]
pub async fn list_projects(State(_state): State<AppState>) -> AppResult<Json<Vec<Project>>> {
    // Phase 1 で実装
    Ok(Json(vec![]))
}

#[utoipa::path(
    post,
    path = "/api/projects",
    request_body = CreateProjectRequest,
    responses(
        (status = 201, description = "Project created", body = Project)
    ),
    tag = "projects"
)]
pub async fn create_project(
    State(_state): State<AppState>,
    Json(_body): Json<CreateProjectRequest>,
) -> AppResult<(axum::http::StatusCode, Json<Project>)> {
    // Phase 1 で実装
    Err(crate::error::AppError::Internal(anyhow::anyhow!(
        "not implemented"
    )))
}

#[utoipa::path(
    get,
    path = "/api/projects/{id}",
    params(("id" = Uuid, Path, description = "Project ID")),
    responses(
        (status = 200, description = "Project found", body = Project),
        (status = 404, description = "Project not found")
    ),
    tag = "projects"
)]
pub async fn get_project(
    State(_state): State<AppState>,
    Path(_id): Path<Uuid>,
) -> AppResult<Json<Project>> {
    // Phase 1 で実装
    Err(crate::error::AppError::NotFound("project not found".into()))
}

#[utoipa::path(
    patch,
    path = "/api/projects/{id}",
    params(("id" = Uuid, Path, description = "Project ID")),
    request_body = UpdateProjectRequest,
    responses(
        (status = 200, description = "Project updated", body = Project),
        (status = 404, description = "Project not found")
    ),
    tag = "projects"
)]
pub async fn update_project(
    State(_state): State<AppState>,
    Path(_id): Path<Uuid>,
    Json(_body): Json<UpdateProjectRequest>,
) -> AppResult<Json<Project>> {
    // Phase 1 で実装
    Err(crate::error::AppError::NotFound("project not found".into()))
}

#[utoipa::path(
    delete,
    path = "/api/projects/{id}",
    params(("id" = Uuid, Path, description = "Project ID")),
    responses(
        (status = 204, description = "Project deleted"),
        (status = 404, description = "Project not found")
    ),
    tag = "projects"
)]
pub async fn delete_project(
    State(_state): State<AppState>,
    Path(_id): Path<Uuid>,
) -> AppResult<axum::http::StatusCode> {
    // Phase 1 で実装
    Err(crate::error::AppError::NotFound("project not found".into()))
}
