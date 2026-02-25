//! Read-only operations for agent sessions.

use sqlx::{SqliteConnection, SqlitePool};
use uuid::Uuid;

use super::AgentSessionRow;
use crate::error::{AppError, AppResult};
use crate::models::agent_session::AgentSession;

pub async fn get_agent_session(
    pool: &SqlitePool,
    task_id: Uuid,
    id: Uuid,
) -> AppResult<AgentSession> {
    let row = sqlx::query_as::<_, AgentSessionRow>(
        r#"
        SELECT id, task_id, agent_type, status, prompt, worktree_name, started_at, finished_at, created_at, updated_at
        FROM agent_sessions
        WHERE id = $1 AND task_id = $2
        "#,
    )
    .bind(id.to_string())
    .bind(task_id.to_string())
    .fetch_optional(pool)
    .await?;

    row.map(|r| r.try_into())
        .transpose()
        .map_err(|e: uuid::Error| AppError::Internal(e.to_string()))?
        .ok_or_else(|| AppError::NotFound(format!("agent session {} not found", id)))
}

pub async fn list_agent_sessions(pool: &SqlitePool, task_id: Uuid) -> AppResult<Vec<AgentSession>> {
    let rows = sqlx::query_as::<_, AgentSessionRow>(
        r#"
        SELECT id, task_id, agent_type, status, prompt, worktree_name, started_at, finished_at, created_at, updated_at
        FROM agent_sessions
        WHERE task_id = $1
        ORDER BY created_at ASC
        "#,
    )
    .bind(task_id.to_string())
    .fetch_all(pool)
    .await?;

    rows.into_iter()
        .map(|r| r.try_into())
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e: uuid::Error| AppError::Internal(e.to_string()))
}

/// List sessions for a task that have a worktree and are in a terminal state.
/// Only returns sessions that are completed, failed, or cancelled — never running/pending ones.
pub async fn list_sessions_with_worktrees(
    pool: &SqlitePool,
    task_id: Uuid,
) -> AppResult<Vec<AgentSession>> {
    let rows = sqlx::query_as::<_, AgentSessionRow>(
        r#"
        SELECT id, task_id, agent_type, status, prompt, worktree_name, started_at, finished_at, created_at, updated_at
        FROM agent_sessions
        WHERE task_id = $1
          AND worktree_name IS NOT NULL
          AND status IN ('completed', 'failed', 'cancelled')
        "#,
    )
    .bind(task_id.to_string())
    .fetch_all(pool)
    .await?;

    rows.into_iter()
        .map(|r| r.try_into())
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e: uuid::Error| AppError::Internal(e.to_string()))
}

/// Check that no active (pending/running) session exists for a task, using a connection (for transactions).
pub async fn check_no_active_session_tx(
    conn: &mut SqliteConnection,
    task_id: Uuid,
) -> AppResult<()> {
    let row = sqlx::query_as::<_, AgentSessionRow>(
        r#"
        SELECT id, task_id, agent_type, status, prompt, worktree_name, started_at, finished_at, created_at, updated_at
        FROM agent_sessions
        WHERE task_id = $1 AND status IN ('pending', 'running', 'paused')
        LIMIT 1
        "#,
    )
    .bind(task_id.to_string())
    .fetch_optional(&mut *conn)
    .await?;

    if row.is_some() {
        return Err(AppError::Conflict(format!(
            "task {task_id} already has an active agent session"
        )));
    }
    Ok(())
}
