use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::Json;
use serde::Deserialize;
use uuid::Uuid;

use garde::Validate;

use crate::agent::orchestrator::StartSessionRequest;
use crate::auth::middleware::AuthUser;
use crate::error::{AppError, AppResult};
use crate::models::agent_session::{
    AgentSession, CreateAgentSessionRequest, StartAgentSessionRequest, StartAgentSessionResponse,
    UpdateAgentSessionRequest,
};
use crate::models::agent_session_output::AgentSessionOutput;
use crate::services::{agent_session_output_service, agent_session_service, authorization_service};
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
    authorization_service::authorize_task(&state.pool, auth.user_id, task_id).await?;
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
    authorization_service::authorize_task(&state.pool, auth.user_id, task_id).await?;
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
    authorization_service::authorize_task(&state.pool, auth.user_id, task_id).await?;
    let session =
        agent_session_service::get_agent_session(&state.pool, task_id, session_id).await?;
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
        (status = 400, description = "Invalid status transition"),
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
    authorization_service::authorize_task(&state.pool, auth.user_id, task_id).await?;
    let session =
        agent_session_service::update_agent_session(&state.pool, task_id, session_id, &body)
            .await?;
    Ok(Json(session))
}

#[utoipa::path(
    post,
    path = "/api/tasks/{task_id}/sessions/start",
    params(("task_id" = Uuid, Path, description = "Task ID")),
    request_body = StartAgentSessionRequest,
    responses(
        (status = 201, description = "Agent session started", body = StartAgentSessionResponse),
        (status = 400, description = "Validation error"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Task not found"),
        (status = 409, description = "Active session already exists")
    ),
    tag = "agent-sessions"
)]
pub async fn start_agent_session(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(task_id): Path<Uuid>,
    Json(body): Json<StartAgentSessionRequest>,
) -> AppResult<(StatusCode, Json<StartAgentSessionResponse>)> {
    body.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;
    authorization_service::authorize_task(&state.pool, auth.user_id, task_id).await?;

    let result = state
        .orchestrator
        .start_session(StartSessionRequest {
            task_id,
            agent_type: body.agent_type,
            prompt: body.prompt,
        })
        .await?;

    let session =
        agent_session_service::get_agent_session(&state.pool, task_id, result.session_id).await?;
    Ok((
        StatusCode::CREATED,
        Json(StartAgentSessionResponse { session }),
    ))
}

#[utoipa::path(
    post,
    path = "/api/tasks/{task_id}/sessions/{session_id}/stop",
    params(
        ("task_id" = Uuid, Path, description = "Task ID"),
        ("session_id" = Uuid, Path, description = "Agent session ID")
    ),
    responses(
        (status = 200, description = "Agent session stopped", body = AgentSession),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Session not found or not running")
    ),
    tag = "agent-sessions"
)]
pub async fn stop_agent_session(
    State(state): State<AppState>,
    auth: AuthUser,
    Path((task_id, session_id)): Path<(Uuid, Uuid)>,
) -> AppResult<Json<AgentSession>> {
    authorization_service::authorize_task(&state.pool, auth.user_id, task_id).await?;

    // Ensure the session belongs to this task before attempting to stop it
    agent_session_service::get_agent_session(&state.pool, task_id, session_id).await?;

    state.orchestrator.stop_session(task_id, session_id).await?;

    let session =
        agent_session_service::get_agent_session(&state.pool, task_id, session_id).await?;
    Ok(Json(session))
}

#[utoipa::path(
    post,
    path = "/api/tasks/{task_id}/sessions/{session_id}/restart",
    params(
        ("task_id" = Uuid, Path, description = "Task ID"),
        ("session_id" = Uuid, Path, description = "Agent session ID to restart")
    ),
    responses(
        (status = 201, description = "New agent session started", body = StartAgentSessionResponse),
        (status = 400, description = "No prompt saved for session"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Task or session not found"),
        (status = 409, description = "Active session already exists")
    ),
    tag = "agent-sessions"
)]
pub async fn restart_agent_session(
    State(state): State<AppState>,
    auth: AuthUser,
    Path((task_id, session_id)): Path<(Uuid, Uuid)>,
) -> AppResult<(StatusCode, Json<StartAgentSessionResponse>)> {
    authorization_service::authorize_task(&state.pool, auth.user_id, task_id).await?;

    let old_session =
        agent_session_service::get_agent_session(&state.pool, task_id, session_id).await?;
    let prompt = old_session.prompt.ok_or_else(|| {
        AppError::Validation("no prompt saved for this session — cannot restart".to_string())
    })?;

    let result = state
        .orchestrator
        .start_session(StartSessionRequest {
            task_id,
            agent_type: old_session.agent_type,
            prompt,
        })
        .await?;

    let session =
        agent_session_service::get_agent_session(&state.pool, task_id, result.session_id).await?;
    Ok((
        StatusCode::CREATED,
        Json(StartAgentSessionResponse { session }),
    ))
}

#[derive(Debug, Deserialize)]
pub struct GetOutputsQuery {
    pub after: Option<i64>,
}

#[utoipa::path(
    get,
    path = "/api/tasks/{task_id}/sessions/{session_id}/outputs",
    params(
        ("task_id" = Uuid, Path, description = "Task ID"),
        ("session_id" = Uuid, Path, description = "Agent session ID"),
        ("after" = Option<i64>, Query, description = "Return outputs after this sequence number")
    ),
    responses(
        (status = 200, description = "Session outputs", body = Vec<AgentSessionOutput>),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Task or session not found")
    ),
    tag = "agent-sessions"
)]
pub async fn get_agent_session_outputs(
    State(state): State<AppState>,
    auth: AuthUser,
    Path((task_id, session_id)): Path<(Uuid, Uuid)>,
    Query(query): Query<GetOutputsQuery>,
) -> AppResult<Json<Vec<AgentSessionOutput>>> {
    authorization_service::authorize_task(&state.pool, auth.user_id, task_id).await?;
    agent_session_service::get_agent_session(&state.pool, task_id, session_id).await?;

    let outputs = match query.after {
        Some(after_seq) => {
            agent_session_output_service::get_outputs_after(&state.pool, session_id, after_seq)
                .await?
        }
        None => agent_session_output_service::get_outputs(&state.pool, session_id).await?,
    };

    Ok(Json(outputs))
}
