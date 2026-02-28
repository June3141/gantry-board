//! Read-only operations for agent sessions.

use sqlx::{SqliteConnection, SqlitePool};
use uuid::Uuid;

use crate::error::{AppError, AppResult};
use crate::models::agent_session::AgentSession;
use crate::repositories::agent_session_repository;

pub async fn get_agent_session(
    pool: &SqlitePool,
    task_id: Uuid,
    id: Uuid,
) -> AppResult<AgentSession> {
    agent_session_repository::find_by_id(pool, task_id, id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("agent session {} not found", id)))
}

pub async fn list_agent_sessions(pool: &SqlitePool, task_id: Uuid) -> AppResult<Vec<AgentSession>> {
    agent_session_repository::find_all_by_task(pool, task_id).await
}

/// List sessions for a task that have a worktree and are in a terminal state.
/// Only returns sessions that are completed, failed, or cancelled — never running/pending ones.
pub async fn list_sessions_with_worktrees(
    pool: &SqlitePool,
    task_id: Uuid,
) -> AppResult<Vec<AgentSession>> {
    agent_session_repository::find_with_worktrees_terminal(pool, task_id).await
}

/// Check that no active (pending/running) session exists for a task, using a connection (for transactions).
pub async fn check_no_active_session_tx(
    conn: &mut SqliteConnection,
    task_id: Uuid,
) -> AppResult<()> {
    let active = agent_session_repository::find_active_by_task_tx(conn, task_id).await?;

    if active.is_some() {
        return Err(AppError::Conflict(format!(
            "task {task_id} already has an active agent session"
        )));
    }
    Ok(())
}
