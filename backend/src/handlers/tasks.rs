use axum::extract::{Path, State};
use axum::Json;
use uuid::Uuid;

use crate::error::AppResult;
use crate::models::task::{CreateTaskRequest, Task, UpdateTaskRequest};
use crate::AppState;

#[utoipa::path(
    get,
    path = "/api/tasks",
    responses(
        (status = 200, description = "List all tasks", body = Vec<Task>)
    ),
    tag = "tasks"
)]
pub async fn list_tasks(State(_state): State<AppState>) -> AppResult<Json<Vec<Task>>> {
    // Phase 1 で実装
    Ok(Json(vec![]))
}

#[utoipa::path(
    post,
    path = "/api/tasks",
    request_body = CreateTaskRequest,
    responses(
        (status = 201, description = "Task created", body = Task)
    ),
    tag = "tasks"
)]
pub async fn create_task(
    State(_state): State<AppState>,
    Json(_body): Json<CreateTaskRequest>,
) -> AppResult<(axum::http::StatusCode, Json<Task>)> {
    // Phase 1 で実装
    Err(crate::error::AppError::Internal(anyhow::anyhow!(
        "not implemented"
    )))
}

#[utoipa::path(
    get,
    path = "/api/tasks/{id}",
    params(("id" = Uuid, Path, description = "Task ID")),
    responses(
        (status = 200, description = "Task found", body = Task),
        (status = 404, description = "Task not found")
    ),
    tag = "tasks"
)]
pub async fn get_task(
    State(_state): State<AppState>,
    Path(_id): Path<Uuid>,
) -> AppResult<Json<Task>> {
    // Phase 1 で実装
    Err(crate::error::AppError::NotFound("task not found".into()))
}

#[utoipa::path(
    patch,
    path = "/api/tasks/{id}",
    params(("id" = Uuid, Path, description = "Task ID")),
    request_body = UpdateTaskRequest,
    responses(
        (status = 200, description = "Task updated", body = Task),
        (status = 404, description = "Task not found")
    ),
    tag = "tasks"
)]
pub async fn update_task(
    State(_state): State<AppState>,
    Path(_id): Path<Uuid>,
    Json(_body): Json<UpdateTaskRequest>,
) -> AppResult<Json<Task>> {
    // Phase 1 で実装
    Err(crate::error::AppError::NotFound("task not found".into()))
}

#[utoipa::path(
    delete,
    path = "/api/tasks/{id}",
    params(("id" = Uuid, Path, description = "Task ID")),
    responses(
        (status = 204, description = "Task deleted"),
        (status = 404, description = "Task not found")
    ),
    tag = "tasks"
)]
pub async fn delete_task(
    State(_state): State<AppState>,
    Path(_id): Path<Uuid>,
) -> AppResult<axum::http::StatusCode> {
    // Phase 1 で実装
    Err(crate::error::AppError::NotFound("task not found".into()))
}
