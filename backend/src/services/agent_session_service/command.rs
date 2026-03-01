//! Write operations for agent sessions.

use chrono::Utc;
use sqlx::{SqliteConnection, SqlitePool};
use uuid::Uuid;

use crate::error::{AppError, AppResult};
use crate::models::agent_session::{
    AgentSession, AgentSessionStatus, CreateAgentSessionRequest, UpdateAgentSessionRequest,
};
use crate::repositories::agent_session_repository;

use super::query::get_agent_session;

pub async fn create_agent_session(
    pool: &SqlitePool,
    task_id: Uuid,
    req: &CreateAgentSessionRequest,
) -> AppResult<AgentSession> {
    let id = Uuid::new_v4();
    let now = Utc::now();

    agent_session_repository::insert(
        pool,
        id,
        task_id,
        &req.agent_type,
        &AgentSessionStatus::Pending,
        now,
    )
    .await?;

    Ok(AgentSession {
        id,
        task_id,
        agent_type: req.agent_type.clone(),
        status: AgentSessionStatus::Pending,
        prompt: None,
        worktree_name: None,
        started_at: None,
        finished_at: None,
        created_at: now,
        updated_at: now,
    })
}

/// Create an agent session using a connection (for transactions).
pub async fn create_agent_session_tx(
    conn: &mut SqliteConnection,
    task_id: Uuid,
    req: &CreateAgentSessionRequest,
) -> AppResult<AgentSession> {
    let id = Uuid::new_v4();
    let now = Utc::now();

    agent_session_repository::insert_tx(
        conn,
        id,
        task_id,
        &req.agent_type,
        &AgentSessionStatus::Pending,
        now,
    )
    .await?;

    Ok(AgentSession {
        id,
        task_id,
        agent_type: req.agent_type.clone(),
        status: AgentSessionStatus::Pending,
        prompt: None,
        worktree_name: None,
        started_at: None,
        finished_at: None,
        created_at: now,
        updated_at: now,
    })
}

pub async fn save_prompt(pool: &SqlitePool, session_id: Uuid, prompt: &str) -> AppResult<()> {
    let affected = agent_session_repository::update_prompt(pool, session_id, prompt).await?;
    if affected == 0 {
        return Err(AppError::NotFound(format!(
            "Agent session {session_id} not found"
        )));
    }
    Ok(())
}

/// Save prompt for a session using a connection (for transactions).
pub async fn save_prompt_tx(
    conn: &mut SqliteConnection,
    session_id: Uuid,
    prompt: &str,
) -> AppResult<()> {
    let affected = agent_session_repository::update_prompt_tx(conn, session_id, prompt).await?;
    if affected == 0 {
        return Err(AppError::NotFound(format!(
            "Agent session {session_id} not found"
        )));
    }
    Ok(())
}

/// Save worktree name for a session.
pub async fn save_worktree_name(
    pool: &SqlitePool,
    session_id: Uuid,
    worktree_name: &str,
) -> AppResult<()> {
    let affected =
        agent_session_repository::update_worktree_name(pool, session_id, worktree_name).await?;
    if affected == 0 {
        return Err(AppError::NotFound(format!(
            "Agent session {session_id} not found"
        )));
    }
    Ok(())
}

/// Clear the worktree_name for a session after cleanup.
pub async fn clear_worktree_name(pool: &SqlitePool, session_id: Uuid) -> AppResult<()> {
    agent_session_repository::clear_worktree_name(pool, session_id).await
}

/// Re-export RecoveredSession from the repository.
pub use agent_session_repository::RecoveredSession;

/// Recover orphaned agent sessions by marking all active sessions as failed.
///
/// This is called on startup to clean up sessions that were left in an active
/// state (running, paused, pending) due to an unclean shutdown.
///
/// This function is idempotent -- calling it multiple times is safe.
pub async fn recover_orphaned_sessions(pool: &SqlitePool) -> AppResult<Vec<RecoveredSession>> {
    agent_session_repository::recover_orphaned(pool).await
}

fn validate_status_transition(from: &AgentSessionStatus, to: &AgentSessionStatus) -> AppResult<()> {
    use AgentSessionStatus::*;
    let allowed = matches!(
        (from, to),
        (Pending, Running)
            | (Pending, Cancelled)
            | (Running, Completed)
            | (Running, Failed)
            | (Running, Cancelled)
            | (Running, Paused)
            | (Paused, Running)
            | (Paused, Cancelled)
            | (Paused, Completed)
            | (Paused, Failed)
    );
    if !allowed {
        return Err(AppError::Validation(format!(
            "invalid status transition from {} to {}",
            serde_json::to_value(from)
                .unwrap_or_default()
                .as_str()
                .unwrap_or("unknown"),
            serde_json::to_value(to)
                .unwrap_or_default()
                .as_str()
                .unwrap_or("unknown"),
        )));
    }
    Ok(())
}

#[tracing::instrument(skip(pool, req), fields(session_id = %id, %task_id))]
pub async fn update_agent_session(
    pool: &SqlitePool,
    task_id: Uuid,
    id: Uuid,
    req: &UpdateAgentSessionRequest,
) -> AppResult<AgentSession> {
    let existing = get_agent_session(pool, task_id, id).await?;
    validate_status_transition(&existing.status, &req.status)?;
    let now = Utc::now();

    let started_at = match req.status {
        AgentSessionStatus::Running => existing.started_at.or(Some(now)),
        _ => existing.started_at,
    };

    let finished_at = match req.status {
        AgentSessionStatus::Completed
        | AgentSessionStatus::Failed
        | AgentSessionStatus::Cancelled => Some(existing.finished_at.unwrap_or(now)),
        _ => None,
    };

    agent_session_repository::update_status(
        pool,
        id,
        task_id,
        &req.status,
        started_at,
        finished_at,
        now,
    )
    .await?;

    Ok(AgentSession {
        id,
        task_id: existing.task_id,
        agent_type: existing.agent_type,
        status: req.status.clone(),
        prompt: existing.prompt,
        worktree_name: existing.worktree_name,
        started_at,
        finished_at,
        created_at: existing.created_at,
        updated_at: now,
    })
}
