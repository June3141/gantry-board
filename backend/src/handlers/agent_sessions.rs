use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use uuid::Uuid;

use crate::auth::middleware::AuthUser;
use crate::error::AppResult;
use crate::models::agent_session::{
    AgentSession, CreateAgentSessionRequest, UpdateAgentSessionRequest,
};
use crate::services::{agent_session_service, authorization_service, task_service};
use crate::AppState;

#[utoipa::path(
    post,
    path = "/api/tasks/{task_id}/sessions",
    params(("task_id" = Uuid, Path, description = "Task ID")),
    request_body = CreateAgentSessionRequest,
    responses(
        (status = 201, description = "Agent session created", body = AgentSession),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Task not found")
    ),
    tag = "agent-sessions"
)]
pub async fn create_agent_session(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(task_id): Path<Uuid>,
    Json(body): Json<CreateAgentSessionRequest>,
) -> AppResult<(StatusCode, Json<AgentSession>)> {
    let task = task_service::get_task(&state.pool, task_id).await?;
    authorization_service::require_project_member(&state.pool, auth.user_id, task.project_id)
        .await?;
    let session = agent_session_service::create_agent_session(&state.pool, task_id, &body).await?;
    Ok((StatusCode::CREATED, Json(session)))
}

#[utoipa::path(
    get,
    path = "/api/tasks/{task_id}/sessions",
    params(("task_id" = Uuid, Path, description = "Task ID")),
    responses(
        (status = 200, description = "List agent sessions", body = Vec<AgentSession>),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Task not found")
    ),
    tag = "agent-sessions"
)]
pub async fn list_agent_sessions(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(task_id): Path<Uuid>,
) -> AppResult<Json<Vec<AgentSession>>> {
    let task = task_service::get_task(&state.pool, task_id).await?;
    authorization_service::require_project_member(&state.pool, auth.user_id, task.project_id)
        .await?;
    let sessions = agent_session_service::list_agent_sessions(&state.pool, task_id).await?;
    Ok(Json(sessions))
}

#[utoipa::path(
    get,
    path = "/api/tasks/{task_id}/sessions/{session_id}",
    params(
        ("task_id" = Uuid, Path, description = "Task ID"),
        ("session_id" = Uuid, Path, description = "Agent session ID")
    ),
    responses(
        (status = 200, description = "Agent session found", body = AgentSession),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Agent session not found")
    ),
    tag = "agent-sessions"
)]
pub async fn get_agent_session(
    State(state): State<AppState>,
    auth: AuthUser,
    Path((task_id, session_id)): Path<(Uuid, Uuid)>,
) -> AppResult<Json<AgentSession>> {
    let task = task_service::get_task(&state.pool, task_id).await?;
    authorization_service::require_project_member(&state.pool, auth.user_id, task.project_id)
        .await?;
    let session = agent_session_service::get_agent_session(&state.pool, session_id).await?;
    Ok(Json(session))
}

#[utoipa::path(
    patch,
    path = "/api/tasks/{task_id}/sessions/{session_id}",
    params(
        ("task_id" = Uuid, Path, description = "Task ID"),
        ("session_id" = Uuid, Path, description = "Agent session ID")
    ),
    request_body = UpdateAgentSessionRequest,
    responses(
        (status = 200, description = "Agent session updated", body = AgentSession),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Agent session not found")
    ),
    tag = "agent-sessions"
)]
pub async fn update_agent_session(
    State(state): State<AppState>,
    auth: AuthUser,
    Path((task_id, session_id)): Path<(Uuid, Uuid)>,
    Json(body): Json<UpdateAgentSessionRequest>,
) -> AppResult<Json<AgentSession>> {
    let task = task_service::get_task(&state.pool, task_id).await?;
    authorization_service::require_project_member(&state.pool, auth.user_id, task.project_id)
        .await?;
    let session =
        agent_session_service::update_agent_session(&state.pool, session_id, &body).await?;
    Ok(Json(session))
}
