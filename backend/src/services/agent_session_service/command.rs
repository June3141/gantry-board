//! Write operations for agent sessions.

use chrono::Utc;
use sqlx::{SqliteConnection, SqlitePool};
use uuid::Uuid;

use crate::error::{AppError, AppResult};
use crate::models::agent_session::{
    AgentSession, AgentSessionStatus, CreateAgentSessionRequest, UpdateAgentSessionRequest,
};

use super::query::get_agent_session;

pub async fn create_agent_session(
    pool: &SqlitePool,
    task_id: Uuid,
    req: &CreateAgentSessionRequest,
) -> AppResult<AgentSession> {
    let id = Uuid::new_v4();
    let now = Utc::now();

    sqlx::query(
        r#"
        INSERT INTO agent_sessions (id, task_id, agent_type, status, created_at, updated_at)
        VALUES ($1, $2, $3, $4, $5, $6)
        "#,
    )
    .bind(id.to_string())
    .bind(task_id.to_string())
    .bind(&req.agent_type)
    .bind(&AgentSessionStatus::Pending)
    .bind(now)
    .bind(now)
    .execute(pool)
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

    sqlx::query(
        r#"
        INSERT INTO agent_sessions (id, task_id, agent_type, status, created_at, updated_at)
        VALUES ($1, $2, $3, $4, $5, $6)
        "#,
    )
    .bind(id.to_string())
    .bind(task_id.to_string())
    .bind(&req.agent_type)
    .bind(&AgentSessionStatus::Pending)
    .bind(now)
    .bind(now)
    .execute(&mut *conn)
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
    let result = sqlx::query(
        "UPDATE agent_sessions SET prompt = $1, updated_at = CURRENT_TIMESTAMP WHERE id = $2",
    )
    .bind(prompt)
    .bind(session_id.to_string())
    .execute(pool)
    .await?;
    if result.rows_affected() == 0 {
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
    let result = sqlx::query(
        "UPDATE agent_sessions SET prompt = $1, updated_at = CURRENT_TIMESTAMP WHERE id = $2",
    )
    .bind(prompt)
    .bind(session_id.to_string())
    .execute(&mut *conn)
    .await?;
    if result.rows_affected() == 0 {
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
    let result = sqlx::query(
        "UPDATE agent_sessions SET worktree_name = $1, updated_at = CURRENT_TIMESTAMP WHERE id = $2",
    )
    .bind(worktree_name)
    .bind(session_id.to_string())
    .execute(pool)
    .await?;
    if result.rows_affected() == 0 {
        return Err(AppError::NotFound(format!(
            "Agent session {session_id} not found"
        )));
    }
    Ok(())
}

/// Clear the worktree_name for a session after cleanup.
pub async fn clear_worktree_name(pool: &SqlitePool, session_id: Uuid) -> AppResult<()> {
    sqlx::query(
        "UPDATE agent_sessions SET worktree_name = NULL, updated_at = CURRENT_TIMESTAMP WHERE id = $1",
    )
    .bind(session_id.to_string())
    .execute(pool)
    .await?;
    Ok(())
}

/// Recovered session info for logging and worktree cleanup.
#[derive(Debug)]
pub struct RecoveredSession {
    pub id: String,
    pub task_id: String,
    pub worktree_name: Option<String>,
}

/// Recover orphaned agent sessions by marking all active sessions as failed.
///
/// This is called on startup to clean up sessions that were left in an active
/// state (running, paused, pending) due to an unclean shutdown.
///
/// This function is idempotent -- calling it multiple times is safe.
pub async fn recover_orphaned_sessions(pool: &SqlitePool) -> AppResult<Vec<RecoveredSession>> {
    let rows = sqlx::query_as::<_, (String, String, Option<String>)>(
        r#"
        UPDATE agent_sessions
        SET status = 'failed', finished_at = CURRENT_TIMESTAMP, updated_at = CURRENT_TIMESTAMP
        WHERE status IN ('running', 'paused', 'pending')
        RETURNING id, task_id, worktree_name
        "#,
    )
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(|(id, task_id, worktree_name)| RecoveredSession {
            id,
            task_id,
            worktree_name,
        })
        .collect())
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

    sqlx::query(
        r#"
        UPDATE agent_sessions
        SET status = $1, started_at = $2, finished_at = $3, updated_at = $4
        WHERE id = $5 AND task_id = $6
        "#,
    )
    .bind(&req.status)
    .bind(started_at)
    .bind(finished_at)
    .bind(now)
    .bind(id.to_string())
    .bind(task_id.to_string())
    .execute(pool)
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
