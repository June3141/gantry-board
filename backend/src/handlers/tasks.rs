use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::Json;
use garde::Validate;
use serde::Deserialize;
use uuid::Uuid;

use crate::auth::middleware::AuthUser;
use crate::error::{AppError, AppResult};
use crate::models::pagination::{self, PaginatedResponse};
use crate::models::task::{CreateTaskRequest, Task, UpdateTaskRequest};
use crate::realtime::event::SseEvent;
use crate::services::{authorization_service, task_service};
use crate::AppState;

/// Fire-and-forget auto-push of a task to GitHub when a link exists.
fn spawn_auto_push(state: &AppState, task: &Task) {
    let github_client = state
        .github_client
        .as_ref()
        .cloned()
        .unwrap_or_else(|| std::sync::Arc::new(crate::github::api::NoopGitHubClient));
    let pool = state.pool.clone();
    let task = task.clone();
    tokio::spawn(async move {
        let engine = crate::github::sync_engine::SyncEngine::new(github_client, pool);
        if let Err(e) = engine.try_push_task(&task).await {
            tracing::warn!(task_id = %task.id, error = %e, "auto-push to GitHub failed");
        }
    });
}

#[derive(Debug, Deserialize)]
pub struct ListTasksQuery {
    pub project_id: Uuid,
    #[serde(default = "pagination::default_limit")]
    pub limit: i64,
    #[serde(default)]
    pub offset: i64,
}

#[utoipa::path(
    get,
    path = "/api/tasks",
    params(
        ("project_id" = Uuid, Query, description = "Filter by project ID"),
        ("limit" = Option<i64>, Query, description = "Maximum number of items to return (default 50)"),
        ("offset" = Option<i64>, Query, description = "Number of items to skip (default 0)"),
    ),
    responses(
        (status = 200, description = "List tasks for project", body = PaginatedResponse<Task>),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Project not found")
    ),
    tag = "tasks"
)]
pub async fn list_tasks(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(query): Query<ListTasksQuery>,
) -> AppResult<Json<PaginatedResponse<Task>>> {
    pagination::validate(query.limit, query.offset)?;
    authorization_service::authorize_project(&state.pool, auth.user_id, query.project_id).await?;
    let (tasks, total) = task_service::list_tasks_paginated(
        &state.pool,
        query.project_id,
        query.limit,
        query.offset,
    )
    .await?;
    Ok(Json(PaginatedResponse {
        data: tasks,
        total,
        limit: query.limit,
        offset: query.offset,
    }))
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
    authorization_service::authorize_project(&state.pool, auth.user_id, body.project_id).await?;
    let task = task_service::create_task(&state.pool, &body).await?;
    state
        .sse_hub
        .broadcast(SseEvent::task_created(task.clone()));
    spawn_auto_push(&state, &task);
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
    let task = authorization_service::authorize_task(&state.pool, auth.user_id, id).await?;
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
    authorization_service::authorize_task(&state.pool, auth.user_id, id).await?;
    let task = task_service::update_task(&state.pool, id, &body).await?;
    state
        .sse_hub
        .broadcast(SseEvent::task_updated(task.clone()));
    spawn_auto_push(&state, &task);
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
    authorization_service::authorize_task(&state.pool, auth.user_id, id).await?;
    task_service::delete_task(&state.pool, id).await?;
    state.sse_hub.broadcast(SseEvent::task_deleted(id));
    Ok(StatusCode::NO_CONTENT)
}
