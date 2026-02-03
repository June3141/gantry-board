use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use garde::Validate;
use uuid::Uuid;

use crate::error::{AppError, AppResult};
use crate::models::project::{CreateProjectRequest, Project, UpdateProjectRequest};
use crate::services::project_service;
use crate::AppState;

#[utoipa::path(
    get,
    path = "/api/projects",
    responses(
        (status = 200, description = "List all projects", body = Vec<Project>)
    ),
    tag = "projects"
)]
pub async fn list_projects(State(state): State<AppState>) -> AppResult<Json<Vec<Project>>> {
    let projects = project_service::list_projects(&state.pool).await?;
    Ok(Json(projects))
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
    State(state): State<AppState>,
    Json(body): Json<CreateProjectRequest>,
) -> AppResult<(StatusCode, Json<Project>)> {
    body.validate().map_err(|e| AppError::Validation(e.to_string()))?;
    let project = project_service::create_project(&state.pool, &body).await?;
    Ok((StatusCode::CREATED, Json(project)))
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
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> AppResult<Json<Project>> {
    let project = project_service::get_project(&state.pool, id).await?;
    Ok(Json(project))
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
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(body): Json<UpdateProjectRequest>,
) -> AppResult<Json<Project>> {
    body.validate().map_err(|e| AppError::Validation(e.to_string()))?;
    let project = project_service::update_project(&state.pool, id, &body).await?;
    Ok(Json(project))
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
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> AppResult<StatusCode> {
    project_service::delete_project(&state.pool, id).await?;
    Ok(StatusCode::NO_CONTENT)
}
