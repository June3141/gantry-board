use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use garde::Validate;
use uuid::Uuid;

use crate::auth::middleware::AuthUser;
use crate::error::{AppError, AppResult};
use crate::models::task_comment::{CreateCommentRequest, TaskComment, UpdateCommentRequest};
use crate::services::{authorization_service, task_comment_service, task_service};
use crate::sse::event::SseEvent;
use crate::AppState;

#[utoipa::path(
    post,
    path = "/api/tasks/{task_id}/comments",
    params(("task_id" = Uuid, Path, description = "Task ID")),
    request_body = CreateCommentRequest,
    responses(
        (status = 201, description = "Comment created", body = TaskComment),
        (status = 400, description = "Validation error"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Task not found")
    ),
    tag = "task-comments"
)]
pub async fn create_comment(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(task_id): Path<Uuid>,
    Json(body): Json<CreateCommentRequest>,
) -> AppResult<(StatusCode, Json<TaskComment>)> {
    body.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;
    let task = task_service::get_task(&state.pool, task_id).await?;
    authorization_service::require_project_member(&state.pool, auth.user_id, task.project_id)
        .await?;
    let comment =
        task_comment_service::create_comment(&state.pool, task_id, auth.user_id, &body).await?;
    state
        .sse_hub
        .broadcast(SseEvent::comment_created(comment.clone()));
    Ok((StatusCode::CREATED, Json(comment)))
}

#[utoipa::path(
    get,
    path = "/api/tasks/{task_id}/comments",
    params(("task_id" = Uuid, Path, description = "Task ID")),
    responses(
        (status = 200, description = "List comments", body = Vec<TaskComment>),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Task not found")
    ),
    tag = "task-comments"
)]
pub async fn list_comments(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(task_id): Path<Uuid>,
) -> AppResult<Json<Vec<TaskComment>>> {
    let task = task_service::get_task(&state.pool, task_id).await?;
    authorization_service::require_project_member(&state.pool, auth.user_id, task.project_id)
        .await?;
    let comments = task_comment_service::list_comments(&state.pool, task_id).await?;
    Ok(Json(comments))
}

#[utoipa::path(
    patch,
    path = "/api/tasks/{task_id}/comments/{comment_id}",
    params(
        ("task_id" = Uuid, Path, description = "Task ID"),
        ("comment_id" = Uuid, Path, description = "Comment ID")
    ),
    request_body = UpdateCommentRequest,
    responses(
        (status = 200, description = "Comment updated", body = TaskComment),
        (status = 400, description = "Validation error"),
        (status = 403, description = "Forbidden - only author can edit"),
        (status = 404, description = "Comment not found")
    ),
    tag = "task-comments"
)]
pub async fn update_comment(
    State(state): State<AppState>,
    auth: AuthUser,
    Path((task_id, comment_id)): Path<(Uuid, Uuid)>,
    Json(body): Json<UpdateCommentRequest>,
) -> AppResult<Json<TaskComment>> {
    body.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;
    let task = task_service::get_task(&state.pool, task_id).await?;
    authorization_service::require_project_member(&state.pool, auth.user_id, task.project_id)
        .await?;
    let comment =
        task_comment_service::update_comment(&state.pool, comment_id, auth.user_id, &body).await?;
    state
        .sse_hub
        .broadcast(SseEvent::comment_updated(comment.clone()));
    Ok(Json(comment))
}

#[utoipa::path(
    delete,
    path = "/api/tasks/{task_id}/comments/{comment_id}",
    params(
        ("task_id" = Uuid, Path, description = "Task ID"),
        ("comment_id" = Uuid, Path, description = "Comment ID")
    ),
    responses(
        (status = 204, description = "Comment deleted"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Comment not found")
    ),
    tag = "task-comments"
)]
pub async fn delete_comment(
    State(state): State<AppState>,
    auth: AuthUser,
    Path((_task_id, comment_id)): Path<(Uuid, Uuid)>,
) -> AppResult<StatusCode> {
    let existing = task_comment_service::get_comment(&state.pool, comment_id).await?;
    if existing.user_id == auth.user_id {
        // Author can delete their own comment — skip re-fetch inside the service
        task_comment_service::delete_comment_admin(&state.pool, comment_id).await?;
    } else {
        // Non-author: authorize using the comment's actual task/project
        let task = task_service::get_task(&state.pool, existing.task_id).await?;
        authorization_service::require_project_admin(&state.pool, auth.user_id, task.project_id)
            .await?;
        task_comment_service::delete_comment_admin(&state.pool, comment_id).await?;
    }
    state
        .sse_hub
        .broadcast(SseEvent::comment_deleted(comment_id, existing.task_id));
    Ok(StatusCode::NO_CONTENT)
}
