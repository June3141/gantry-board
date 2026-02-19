use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::Json;
use chrono::{DateTime, Utc};
use garde::Validate;
use serde::Deserialize;
use uuid::Uuid;

use crate::auth::middleware::AuthUser;
use crate::error::{AppError, AppResult};
use crate::models::project_message::{CreateMessageRequest, ProjectMessage};
use crate::services::{authorization_service, project_message_service};
use crate::sse::event::SseEvent;
use crate::AppState;

const DEFAULT_PAGE_SIZE: i64 = 50;

#[derive(Debug, Deserialize)]
pub struct ListMessagesQuery {
    pub before: Option<DateTime<Utc>>,
    pub limit: Option<i64>,
}

#[utoipa::path(
    get,
    path = "/api/projects/{project_id}/messages",
    params(
        ("project_id" = Uuid, Path, description = "Project ID"),
        ("before" = Option<String>, Query, description = "Cursor: return messages before this timestamp"),
        ("limit" = Option<i64>, Query, description = "Max messages to return (default 50, max 100)")
    ),
    responses(
        (status = 200, description = "List project messages", body = Vec<ProjectMessage>),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Project not found")
    ),
    tag = "project-messages"
)]
pub async fn list_messages(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(project_id): Path<Uuid>,
    Query(query): Query<ListMessagesQuery>,
) -> AppResult<Json<Vec<ProjectMessage>>> {
    authorization_service::authorize_project(&state.pool, auth.user_id, project_id).await?;
    let limit = query.limit.unwrap_or(DEFAULT_PAGE_SIZE).clamp(1, 100);
    let messages =
        project_message_service::list_messages(&state.pool, project_id, query.before, limit)
            .await?;
    Ok(Json(messages))
}

#[utoipa::path(
    post,
    path = "/api/projects/{project_id}/messages",
    params(("project_id" = Uuid, Path, description = "Project ID")),
    request_body = CreateMessageRequest,
    responses(
        (status = 201, description = "Message created", body = ProjectMessage),
        (status = 400, description = "Validation error"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Project not found")
    ),
    tag = "project-messages"
)]
pub async fn create_message(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(project_id): Path<Uuid>,
    Json(body): Json<CreateMessageRequest>,
) -> AppResult<(StatusCode, Json<ProjectMessage>)> {
    body.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;
    authorization_service::authorize_project(&state.pool, auth.user_id, project_id).await?;
    let message =
        project_message_service::create_message(&state.pool, project_id, auth.user_id, &body)
            .await?;
    state
        .sse_hub
        .broadcast(SseEvent::project_message_created(message.clone()));
    Ok((StatusCode::CREATED, Json(message)))
}

#[utoipa::path(
    delete,
    path = "/api/projects/{project_id}/messages/{message_id}",
    params(
        ("project_id" = Uuid, Path, description = "Project ID"),
        ("message_id" = Uuid, Path, description = "Message ID")
    ),
    responses(
        (status = 204, description = "Message deleted"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Message not found")
    ),
    tag = "project-messages"
)]
pub async fn delete_message(
    State(state): State<AppState>,
    auth: AuthUser,
    Path((project_id, message_id)): Path<(Uuid, Uuid)>,
) -> AppResult<StatusCode> {
    authorization_service::authorize_project(&state.pool, auth.user_id, project_id).await?;

    let existing = project_message_service::get_message(&state.pool, message_id).await?;
    if existing.user_id == auth.user_id {
        project_message_service::delete_message_admin(&state.pool, message_id).await?;
    } else {
        authorization_service::require_project_admin(&state.pool, auth.user_id, project_id).await?;
        project_message_service::delete_message_admin(&state.pool, message_id).await?;
    }

    state
        .sse_hub
        .broadcast(SseEvent::project_message_deleted(message_id, project_id));
    Ok(StatusCode::NO_CONTENT)
}
