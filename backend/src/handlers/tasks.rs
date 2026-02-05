use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::Json;
use garde::Validate;
use serde::Deserialize;
use uuid::Uuid;

use crate::auth::middleware::AuthUser;
use crate::error::{AppError, AppResult};
use crate::models::task::{CreateTaskRequest, Task, UpdateTaskRequest};
use crate::services::{authorization_service, project_service, task_service};
use crate::sse::event::SseEvent;
use crate::AppState;

#[derive(Debug, Deserialize)]
pub struct ListTasksQuery {
    pub project_id: Uuid,
}

#[utoipa::path(
    get,
    path = "/api/tasks",
    params(("project_id" = Uuid, Query, description = "Filter by project ID")),
    responses(
        (status = 200, description = "List tasks for project", body = Vec<Task>),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Project not found")
    ),
    tag = "tasks"
)]
pub async fn list_tasks(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(query): Query<ListTasksQuery>,
) -> AppResult<Json<Vec<Task>>> {
    project_service::get_project(&state.pool, query.project_id).await?;
    authorization_service::require_project_member(&state.pool, auth.user_id, query.project_id)
        .await?;
    let tasks = task_service::list_tasks(&state.pool, query.project_id).await?;
    Ok(Json(tasks))
}

#[utoipa::path(
    post,
    path = "/api/tasks",
    request_body = CreateTaskRequest,
    responses(
        (status = 201, description = "Task created", body = Task),
        (status = 400, description = "Validation error"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Project not found")
    ),
    tag = "tasks"
)]
pub async fn create_task(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(body): Json<CreateTaskRequest>,
) -> AppResult<(StatusCode, Json<Task>)> {
    body.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;
    project_service::get_project(&state.pool, body.project_id).await?;
    authorization_service::require_project_member(&state.pool, auth.user_id, body.project_id)
        .await?;
    let task = task_service::create_task(&state.pool, &body).await?;
    state
        .sse_hub
        .broadcast(SseEvent::task_created(task.clone()));
    Ok((StatusCode::CREATED, Json(task)))
}

#[utoipa::path(
    get,
    path = "/api/tasks/{id}",
    params(("id" = Uuid, Path, description = "Task ID")),
    responses(
        (status = 200, description = "Task found", body = Task),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Task not found")
    ),
    tag = "tasks"
)]
pub async fn get_task(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> AppResult<Json<Task>> {
    let task = task_service::get_task(&state.pool, id).await?;
    authorization_service::require_project_member(&state.pool, auth.user_id, task.project_id)
        .await?;
    Ok(Json(task))
}

#[utoipa::path(
    patch,
    path = "/api/tasks/{id}",
    params(("id" = Uuid, Path, description = "Task ID")),
    request_body = UpdateTaskRequest,
    responses(
        (status = 200, description = "Task updated", body = Task),
        (status = 400, description = "Validation error"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Task not found")
    ),
    tag = "tasks"
)]
pub async fn update_task(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
    Json(body): Json<UpdateTaskRequest>,
) -> AppResult<Json<Task>> {
    body.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;
    let existing = task_service::get_task(&state.pool, id).await?;
    authorization_service::require_project_member(&state.pool, auth.user_id, existing.project_id)
        .await?;
    let task = task_service::update_task(&state.pool, id, &body).await?;
    state
        .sse_hub
        .broadcast(SseEvent::task_updated(task.clone()));
    Ok(Json(task))
}

#[utoipa::path(
    delete,
    path = "/api/tasks/{id}",
    params(("id" = Uuid, Path, description = "Task ID")),
    responses(
        (status = 204, description = "Task deleted"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Task not found")
    ),
    tag = "tasks"
)]
pub async fn delete_task(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> AppResult<StatusCode> {
    let existing = task_service::get_task(&state.pool, id).await?;
    authorization_service::require_project_member(&state.pool, auth.user_id, existing.project_id)
        .await?;
    task_service::delete_task(&state.pool, id).await?;
    state.sse_hub.broadcast(SseEvent::task_deleted(id));
    Ok(StatusCode::NO_CONTENT)
}
