use chrono::{DateTime, Utc};
use sqlx::prelude::FromRow;
use sqlx::{SqliteConnection, SqlitePool};
use uuid::Uuid;

use crate::error::{AppError, AppResult};
use crate::models::agent_session::{
    AgentSession, AgentSessionStatus, AgentType, CreateAgentSessionRequest,
    UpdateAgentSessionRequest,
};

#[derive(FromRow)]
struct AgentSessionRow {
    id: String,
    task_id: String,
    agent_type: AgentType,
    status: AgentSessionStatus,
    prompt: Option<String>,
    worktree_name: Option<String>,
    started_at: Option<DateTime<Utc>>,
    finished_at: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl TryFrom<AgentSessionRow> for AgentSession {
    type Error = uuid::Error;

    fn try_from(row: AgentSessionRow) -> Result<Self, Self::Error> {
        Ok(AgentSession {
            id: row.id.parse()?,
            task_id: row.task_id.parse()?,
            agent_type: row.agent_type,
            status: row.status,
            prompt: row.prompt,
            worktree_name: row.worktree_name,
            started_at: row.started_at,
            finished_at: row.finished_at,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
    }
}

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
/// state (running, paused, pending) due to an unclean shutdown. Sessions in
/// terminal states (completed, failed, cancelled) are not affected.
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::project::CreateProjectRequest;
    use crate::models::task::CreateTaskRequest;
    use crate::services::{project_service, task_service};
    use crate::test_helpers::setup_test_db;

    async fn create_test_task(pool: &SqlitePool) -> (Uuid, Uuid) {
        let project = project_service::create_project(
            pool,
            &CreateProjectRequest {
                name: "Test Project".to_string(),
                description: None,
            },
        )
        .await
        .expect("Failed to create project");

        let task = task_service::create_task(
            pool,
            &CreateTaskRequest {
                project_id: project.id,
                title: "Test Task".to_string(),
                description: None,
                status: None,
                priority: None,
                parent_id: None,
                assigned_to: None,
            },
        )
        .await
        .expect("Failed to create task");

        (project.id, task.id)
    }

    #[tokio::test]
    async fn test_create_agent_session_returns_pending_session() {
        let pool = setup_test_db().await;
        let (_project_id, task_id) = create_test_task(&pool).await;

        let req = CreateAgentSessionRequest {
            agent_type: AgentType::ClaudeCode,
        };
        let session = create_agent_session(&pool, task_id, &req)
            .await
            .expect("Failed to create agent session");

        assert_eq!(session.task_id, task_id);
        assert!(matches!(session.agent_type, AgentType::ClaudeCode));
        assert_eq!(session.status, AgentSessionStatus::Pending);
        assert!(session.started_at.is_none());
        assert!(session.finished_at.is_none());
    }

    #[tokio::test]
    async fn test_create_agent_session_with_gemini_cli() {
        let pool = setup_test_db().await;
        let (_project_id, task_id) = create_test_task(&pool).await;

        let req = CreateAgentSessionRequest {
            agent_type: AgentType::GeminiCli,
        };
        let session = create_agent_session(&pool, task_id, &req)
            .await
            .expect("Failed to create agent session");

        assert!(matches!(session.agent_type, AgentType::GeminiCli));
    }

    #[tokio::test]
    async fn test_create_agent_session_fails_for_nonexistent_task() {
        let pool = setup_test_db().await;
        let fake_task_id = Uuid::new_v4();

        let req = CreateAgentSessionRequest {
            agent_type: AgentType::ClaudeCode,
        };
        let result = create_agent_session(&pool, fake_task_id, &req).await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_get_agent_session_returns_existing() {
        let pool = setup_test_db().await;
        let (_project_id, task_id) = create_test_task(&pool).await;

        let req = CreateAgentSessionRequest {
            agent_type: AgentType::ClaudeCode,
        };
        let created = create_agent_session(&pool, task_id, &req)
            .await
            .expect("Failed to create");

        let session = get_agent_session(&pool, task_id, created.id)
            .await
            .expect("Failed to get");

        assert_eq!(session.id, created.id);
        assert_eq!(session.task_id, task_id);
    }

    #[tokio::test]
    async fn test_get_agent_session_returns_not_found() {
        let pool = setup_test_db().await;
        let (_project_id, task_id) = create_test_task(&pool).await;
        let random_id = Uuid::new_v4();

        let result = get_agent_session(&pool, task_id, random_id).await;

        assert!(matches!(result, Err(AppError::NotFound(_))));
    }

    #[tokio::test]
    async fn test_get_agent_session_wrong_task_returns_not_found() {
        let pool = setup_test_db().await;
        let (_project_id, task_id) = create_test_task(&pool).await;
        let (_project_id2, other_task_id) = create_test_task(&pool).await;

        let created = create_agent_session(
            &pool,
            task_id,
            &CreateAgentSessionRequest {
                agent_type: AgentType::ClaudeCode,
            },
        )
        .await
        .expect("Failed to create");

        let result = get_agent_session(&pool, other_task_id, created.id).await;

        assert!(matches!(result, Err(AppError::NotFound(_))));
    }

    #[tokio::test]
    async fn test_list_agent_sessions_returns_empty() {
        let pool = setup_test_db().await;
        let (_project_id, task_id) = create_test_task(&pool).await;

        let sessions = list_agent_sessions(&pool, task_id)
            .await
            .expect("Failed to list");

        assert!(sessions.is_empty());
    }

    #[tokio::test]
    async fn test_list_agent_sessions_returns_sessions_for_task() {
        let pool = setup_test_db().await;
        let (_project_id, task_id) = create_test_task(&pool).await;

        let req = CreateAgentSessionRequest {
            agent_type: AgentType::ClaudeCode,
        };
        // Create first session and complete it so we can create another
        let session1 = create_agent_session(&pool, task_id, &req)
            .await
            .expect("Failed to create 1");
        update_agent_session(
            &pool,
            task_id,
            session1.id,
            &UpdateAgentSessionRequest {
                status: AgentSessionStatus::Running,
            },
        )
        .await
        .expect("Failed to update to running");
        update_agent_session(
            &pool,
            task_id,
            session1.id,
            &UpdateAgentSessionRequest {
                status: AgentSessionStatus::Completed,
            },
        )
        .await
        .expect("Failed to update to completed");

        // Now create second session (allowed because first is completed)
        create_agent_session(&pool, task_id, &req)
            .await
            .expect("Failed to create 2");

        let sessions = list_agent_sessions(&pool, task_id)
            .await
            .expect("Failed to list");

        assert_eq!(sessions.len(), 2);
    }

    #[tokio::test]
    async fn test_update_status_to_running_sets_started_at() {
        let pool = setup_test_db().await;
        let (_project_id, task_id) = create_test_task(&pool).await;

        let created = create_agent_session(
            &pool,
            task_id,
            &CreateAgentSessionRequest {
                agent_type: AgentType::ClaudeCode,
            },
        )
        .await
        .expect("Failed to create");

        let updated = update_agent_session(
            &pool,
            task_id,
            created.id,
            &UpdateAgentSessionRequest {
                status: AgentSessionStatus::Running,
            },
        )
        .await
        .expect("Failed to update");

        assert_eq!(updated.status, AgentSessionStatus::Running);
        assert!(updated.started_at.is_some());
        assert!(updated.finished_at.is_none());
    }

    #[tokio::test]
    async fn test_update_status_to_completed_sets_finished_at() {
        let pool = setup_test_db().await;
        let (_project_id, task_id) = create_test_task(&pool).await;

        let created = create_agent_session(
            &pool,
            task_id,
            &CreateAgentSessionRequest {
                agent_type: AgentType::ClaudeCode,
            },
        )
        .await
        .expect("Failed to create");

        // First transition to running
        update_agent_session(
            &pool,
            task_id,
            created.id,
            &UpdateAgentSessionRequest {
                status: AgentSessionStatus::Running,
            },
        )
        .await
        .expect("Failed to update to running");

        // Then complete
        let updated = update_agent_session(
            &pool,
            task_id,
            created.id,
            &UpdateAgentSessionRequest {
                status: AgentSessionStatus::Completed,
            },
        )
        .await
        .expect("Failed to update to completed");

        assert_eq!(updated.status, AgentSessionStatus::Completed);
        assert!(updated.started_at.is_some());
        assert!(updated.finished_at.is_some());
    }

    #[tokio::test]
    async fn test_update_status_to_failed_sets_finished_at() {
        let pool = setup_test_db().await;
        let (_project_id, task_id) = create_test_task(&pool).await;

        let created = create_agent_session(
            &pool,
            task_id,
            &CreateAgentSessionRequest {
                agent_type: AgentType::ClaudeCode,
            },
        )
        .await
        .expect("Failed to create");

        // Pending -> Running first
        update_agent_session(
            &pool,
            task_id,
            created.id,
            &UpdateAgentSessionRequest {
                status: AgentSessionStatus::Running,
            },
        )
        .await
        .expect("Failed to update to running");

        let updated = update_agent_session(
            &pool,
            task_id,
            created.id,
            &UpdateAgentSessionRequest {
                status: AgentSessionStatus::Failed,
            },
        )
        .await
        .expect("Failed to update");

        assert_eq!(updated.status, AgentSessionStatus::Failed);
        assert!(updated.finished_at.is_some());
    }

    #[tokio::test]
    async fn test_update_nonexistent_session_returns_not_found() {
        let pool = setup_test_db().await;
        let (_project_id, task_id) = create_test_task(&pool).await;
        let random_id = Uuid::new_v4();

        let result = update_agent_session(
            &pool,
            task_id,
            random_id,
            &UpdateAgentSessionRequest {
                status: AgentSessionStatus::Running,
            },
        )
        .await;

        assert!(matches!(result, Err(AppError::NotFound(_))));
    }

    #[tokio::test]
    async fn test_invalid_transition_completed_to_running_returns_validation_error() {
        let pool = setup_test_db().await;
        let (_project_id, task_id) = create_test_task(&pool).await;

        let created = create_agent_session(
            &pool,
            task_id,
            &CreateAgentSessionRequest {
                agent_type: AgentType::ClaudeCode,
            },
        )
        .await
        .expect("Failed to create");

        // Pending -> Running -> Completed
        update_agent_session(
            &pool,
            task_id,
            created.id,
            &UpdateAgentSessionRequest {
                status: AgentSessionStatus::Running,
            },
        )
        .await
        .expect("Failed");
        update_agent_session(
            &pool,
            task_id,
            created.id,
            &UpdateAgentSessionRequest {
                status: AgentSessionStatus::Completed,
            },
        )
        .await
        .expect("Failed");

        // Completed -> Running should fail
        let result = update_agent_session(
            &pool,
            task_id,
            created.id,
            &UpdateAgentSessionRequest {
                status: AgentSessionStatus::Running,
            },
        )
        .await;

        assert!(matches!(result, Err(AppError::Validation(_))));
    }

    #[tokio::test]
    async fn test_concurrent_pending_sessions_blocked_by_unique_constraint() {
        let pool = setup_test_db().await;
        let (_project_id, task_id) = create_test_task(&pool).await;

        let req = CreateAgentSessionRequest {
            agent_type: AgentType::ClaudeCode,
        };

        // First session: OK
        create_agent_session(&pool, task_id, &req)
            .await
            .expect("First session should succeed");

        // Second session on same task while first is still pending: should fail
        let result = create_agent_session(&pool, task_id, &req).await;
        assert!(result.is_err(), "Second pending session should be rejected");
    }

    #[tokio::test]
    async fn test_new_session_allowed_after_previous_completed() {
        let pool = setup_test_db().await;
        let (_project_id, task_id) = create_test_task(&pool).await;

        let req = CreateAgentSessionRequest {
            agent_type: AgentType::ClaudeCode,
        };

        let session = create_agent_session(&pool, task_id, &req)
            .await
            .expect("First session should succeed");

        // Complete the first session
        update_agent_session(
            &pool,
            task_id,
            session.id,
            &UpdateAgentSessionRequest {
                status: AgentSessionStatus::Running,
            },
        )
        .await
        .expect("Transition to running");
        update_agent_session(
            &pool,
            task_id,
            session.id,
            &UpdateAgentSessionRequest {
                status: AgentSessionStatus::Completed,
            },
        )
        .await
        .expect("Transition to completed");

        // New session should now succeed
        let result = create_agent_session(&pool, task_id, &req).await;
        assert!(
            result.is_ok(),
            "New session after completion should succeed"
        );
    }

    #[tokio::test]
    async fn test_new_session_allowed_after_previous_cancelled() {
        let pool = setup_test_db().await;
        let (_project_id, task_id) = create_test_task(&pool).await;

        let req = CreateAgentSessionRequest {
            agent_type: AgentType::ClaudeCode,
        };

        let session = create_agent_session(&pool, task_id, &req)
            .await
            .expect("First session should succeed");

        // Cancel the first session
        update_agent_session(
            &pool,
            task_id,
            session.id,
            &UpdateAgentSessionRequest {
                status: AgentSessionStatus::Cancelled,
            },
        )
        .await
        .expect("Transition to cancelled");

        // New session should now succeed
        let result = create_agent_session(&pool, task_id, &req).await;
        assert!(
            result.is_ok(),
            "New session after cancellation should succeed"
        );
    }

    #[tokio::test]
    async fn test_running_session_blocks_new_session_creation() {
        let pool = setup_test_db().await;
        let (_project_id, task_id) = create_test_task(&pool).await;

        let req = CreateAgentSessionRequest {
            agent_type: AgentType::ClaudeCode,
        };

        let session = create_agent_session(&pool, task_id, &req)
            .await
            .expect("First session should succeed");

        // Transition to running
        update_agent_session(
            &pool,
            task_id,
            session.id,
            &UpdateAgentSessionRequest {
                status: AgentSessionStatus::Running,
            },
        )
        .await
        .expect("Transition to running");

        // Second session while first is running: should fail
        let result = create_agent_session(&pool, task_id, &req).await;
        assert!(result.is_err(), "Session during running should be rejected");
    }

    #[tokio::test]
    async fn test_new_session_allowed_after_previous_failed() {
        let pool = setup_test_db().await;
        let (_project_id, task_id) = create_test_task(&pool).await;

        let req = CreateAgentSessionRequest {
            agent_type: AgentType::ClaudeCode,
        };

        let session = create_agent_session(&pool, task_id, &req)
            .await
            .expect("First session should succeed");

        // Transition: pending -> running -> failed
        update_agent_session(
            &pool,
            task_id,
            session.id,
            &UpdateAgentSessionRequest {
                status: AgentSessionStatus::Running,
            },
        )
        .await
        .expect("Transition to running");
        update_agent_session(
            &pool,
            task_id,
            session.id,
            &UpdateAgentSessionRequest {
                status: AgentSessionStatus::Failed,
            },
        )
        .await
        .expect("Transition to failed");

        // New session should now succeed
        let result = create_agent_session(&pool, task_id, &req).await;
        assert!(result.is_ok(), "New session after failure should succeed");
    }

    #[tokio::test]
    async fn test_invalid_transition_pending_to_completed_returns_validation_error() {
        let pool = setup_test_db().await;
        let (_project_id, task_id) = create_test_task(&pool).await;

        let created = create_agent_session(
            &pool,
            task_id,
            &CreateAgentSessionRequest {
                agent_type: AgentType::ClaudeCode,
            },
        )
        .await
        .expect("Failed to create");

        // Pending -> Completed should fail (must go through Running)
        let result = update_agent_session(
            &pool,
            task_id,
            created.id,
            &UpdateAgentSessionRequest {
                status: AgentSessionStatus::Completed,
            },
        )
        .await;

        assert!(matches!(result, Err(AppError::Validation(_))));
    }

    #[tokio::test]
    async fn test_transition_running_to_paused_is_valid() {
        let pool = setup_test_db().await;
        let (_project_id, task_id) = create_test_task(&pool).await;

        let created = create_agent_session(
            &pool,
            task_id,
            &CreateAgentSessionRequest {
                agent_type: AgentType::ClaudeCode,
            },
        )
        .await
        .expect("Failed to create");

        update_agent_session(
            &pool,
            task_id,
            created.id,
            &UpdateAgentSessionRequest {
                status: AgentSessionStatus::Running,
            },
        )
        .await
        .expect("Transition to running");

        let paused = update_agent_session(
            &pool,
            task_id,
            created.id,
            &UpdateAgentSessionRequest {
                status: AgentSessionStatus::Paused,
            },
        )
        .await
        .expect("Transition to paused should succeed");

        assert_eq!(paused.status, AgentSessionStatus::Paused);
        assert!(paused.started_at.is_some());
        assert!(paused.finished_at.is_none());
    }

    #[tokio::test]
    async fn test_transition_paused_to_running_is_valid() {
        let pool = setup_test_db().await;
        let (_project_id, task_id) = create_test_task(&pool).await;

        let created = create_agent_session(
            &pool,
            task_id,
            &CreateAgentSessionRequest {
                agent_type: AgentType::ClaudeCode,
            },
        )
        .await
        .expect("Failed to create");

        // Pending → Running → Paused → Running (resume)
        update_agent_session(
            &pool,
            task_id,
            created.id,
            &UpdateAgentSessionRequest {
                status: AgentSessionStatus::Running,
            },
        )
        .await
        .expect("Transition to running");

        update_agent_session(
            &pool,
            task_id,
            created.id,
            &UpdateAgentSessionRequest {
                status: AgentSessionStatus::Paused,
            },
        )
        .await
        .expect("Transition to paused");

        let resumed = update_agent_session(
            &pool,
            task_id,
            created.id,
            &UpdateAgentSessionRequest {
                status: AgentSessionStatus::Running,
            },
        )
        .await
        .expect("Transition from paused to running should succeed");

        assert_eq!(resumed.status, AgentSessionStatus::Running);
        assert!(resumed.finished_at.is_none());
    }

    #[tokio::test]
    async fn test_transition_paused_to_cancelled_is_valid() {
        let pool = setup_test_db().await;
        let (_project_id, task_id) = create_test_task(&pool).await;

        let created = create_agent_session(
            &pool,
            task_id,
            &CreateAgentSessionRequest {
                agent_type: AgentType::ClaudeCode,
            },
        )
        .await
        .expect("Failed to create");

        // Pending → Running → Paused → Cancelled
        update_agent_session(
            &pool,
            task_id,
            created.id,
            &UpdateAgentSessionRequest {
                status: AgentSessionStatus::Running,
            },
        )
        .await
        .expect("Transition to running");

        update_agent_session(
            &pool,
            task_id,
            created.id,
            &UpdateAgentSessionRequest {
                status: AgentSessionStatus::Paused,
            },
        )
        .await
        .expect("Transition to paused");

        let cancelled = update_agent_session(
            &pool,
            task_id,
            created.id,
            &UpdateAgentSessionRequest {
                status: AgentSessionStatus::Cancelled,
            },
        )
        .await
        .expect("Transition from paused to cancelled should succeed");

        assert_eq!(cancelled.status, AgentSessionStatus::Cancelled);
        assert!(cancelled.finished_at.is_some());
    }

    #[tokio::test]
    async fn test_invalid_transition_pending_to_paused() {
        let pool = setup_test_db().await;
        let (_project_id, task_id) = create_test_task(&pool).await;

        let created = create_agent_session(
            &pool,
            task_id,
            &CreateAgentSessionRequest {
                agent_type: AgentType::ClaudeCode,
            },
        )
        .await
        .expect("Failed to create");

        // Pending → Paused should fail
        let result = update_agent_session(
            &pool,
            task_id,
            created.id,
            &UpdateAgentSessionRequest {
                status: AgentSessionStatus::Paused,
            },
        )
        .await;

        assert!(
            matches!(result, Err(AppError::Validation(_))),
            "Pending → Paused should be rejected"
        );
    }

    #[tokio::test]
    async fn test_invalid_transition_completed_to_paused() {
        let pool = setup_test_db().await;
        let (_project_id, task_id) = create_test_task(&pool).await;

        let created = create_agent_session(
            &pool,
            task_id,
            &CreateAgentSessionRequest {
                agent_type: AgentType::ClaudeCode,
            },
        )
        .await
        .expect("Failed to create");

        // Pending → Running → Completed
        update_agent_session(
            &pool,
            task_id,
            created.id,
            &UpdateAgentSessionRequest {
                status: AgentSessionStatus::Running,
            },
        )
        .await
        .expect("Transition to running");
        update_agent_session(
            &pool,
            task_id,
            created.id,
            &UpdateAgentSessionRequest {
                status: AgentSessionStatus::Completed,
            },
        )
        .await
        .expect("Transition to completed");

        // Completed → Paused should fail
        let result = update_agent_session(
            &pool,
            task_id,
            created.id,
            &UpdateAgentSessionRequest {
                status: AgentSessionStatus::Paused,
            },
        )
        .await;

        assert!(
            matches!(result, Err(AppError::Validation(_))),
            "Completed → Paused should be rejected"
        );
    }

    #[tokio::test]
    async fn test_paused_session_blocks_new_session_creation() {
        let pool = setup_test_db().await;
        let (_project_id, task_id) = create_test_task(&pool).await;

        let req = CreateAgentSessionRequest {
            agent_type: AgentType::ClaudeCode,
        };

        let session = create_agent_session(&pool, task_id, &req)
            .await
            .expect("First session should succeed");

        // Pending → Running → Paused
        update_agent_session(
            &pool,
            task_id,
            session.id,
            &UpdateAgentSessionRequest {
                status: AgentSessionStatus::Running,
            },
        )
        .await
        .expect("Transition to running");
        update_agent_session(
            &pool,
            task_id,
            session.id,
            &UpdateAgentSessionRequest {
                status: AgentSessionStatus::Paused,
            },
        )
        .await
        .expect("Transition to paused");

        // New session while paused should fail
        let result = create_agent_session(&pool, task_id, &req).await;
        assert!(
            result.is_err(),
            "New session should be blocked while another is paused"
        );
    }

    #[tokio::test]
    async fn test_recover_orphaned_sessions_marks_active_as_failed() {
        let pool = setup_test_db().await;
        let (_project_id, task_id1) = create_test_task(&pool).await;
        let (_project_id2, task_id2) = create_test_task(&pool).await;
        let (_project_id3, task_id3) = create_test_task(&pool).await;

        let req = CreateAgentSessionRequest {
            agent_type: AgentType::ClaudeCode,
        };

        // Create a running session
        let s1 = create_agent_session(&pool, task_id1, &req)
            .await
            .expect("create s1");
        update_agent_session(
            &pool,
            task_id1,
            s1.id,
            &UpdateAgentSessionRequest {
                status: AgentSessionStatus::Running,
            },
        )
        .await
        .expect("s1 to running");

        // Create a paused session
        let s2 = create_agent_session(&pool, task_id2, &req)
            .await
            .expect("create s2");
        update_agent_session(
            &pool,
            task_id2,
            s2.id,
            &UpdateAgentSessionRequest {
                status: AgentSessionStatus::Running,
            },
        )
        .await
        .expect("s2 to running");
        update_agent_session(
            &pool,
            task_id2,
            s2.id,
            &UpdateAgentSessionRequest {
                status: AgentSessionStatus::Paused,
            },
        )
        .await
        .expect("s2 to paused");

        // Create a pending session
        let s3 = create_agent_session(&pool, task_id3, &req)
            .await
            .expect("create s3");

        // Recover orphaned sessions
        let recovered = recover_orphaned_sessions(&pool)
            .await
            .expect("recover should succeed");
        assert_eq!(
            recovered.len(),
            3,
            "all three active sessions should be recovered"
        );

        // Verify all are now failed with finished_at set
        let r1 = get_agent_session(&pool, task_id1, s1.id)
            .await
            .expect("get s1");
        assert_eq!(r1.status, AgentSessionStatus::Failed);
        assert!(r1.finished_at.is_some());

        let r2 = get_agent_session(&pool, task_id2, s2.id)
            .await
            .expect("get s2");
        assert_eq!(r2.status, AgentSessionStatus::Failed);
        assert!(r2.finished_at.is_some());

        let r3 = get_agent_session(&pool, task_id3, s3.id)
            .await
            .expect("get s3");
        assert_eq!(r3.status, AgentSessionStatus::Failed);
        assert!(r3.finished_at.is_some());
    }

    #[tokio::test]
    async fn test_recover_orphaned_sessions_idempotent() {
        let pool = setup_test_db().await;
        let (_project_id, task_id) = create_test_task(&pool).await;

        let req = CreateAgentSessionRequest {
            agent_type: AgentType::ClaudeCode,
        };

        // Create a running session
        let s = create_agent_session(&pool, task_id, &req)
            .await
            .expect("create session");
        update_agent_session(
            &pool,
            task_id,
            s.id,
            &UpdateAgentSessionRequest {
                status: AgentSessionStatus::Running,
            },
        )
        .await
        .expect("to running");

        // First recovery
        let recovered1 = recover_orphaned_sessions(&pool)
            .await
            .expect("first recover");
        assert_eq!(recovered1.len(), 1);

        // Second recovery should be a no-op
        let recovered2 = recover_orphaned_sessions(&pool)
            .await
            .expect("second recover");
        assert_eq!(recovered2.len(), 0, "second call should be no-op");
    }

    #[tokio::test]
    async fn test_recover_orphaned_sessions_does_not_affect_terminal_states() {
        let pool = setup_test_db().await;
        let (_project_id, task_id1) = create_test_task(&pool).await;
        let (_project_id2, task_id2) = create_test_task(&pool).await;

        let req = CreateAgentSessionRequest {
            agent_type: AgentType::ClaudeCode,
        };

        // Create a completed session
        let s1 = create_agent_session(&pool, task_id1, &req)
            .await
            .expect("create s1");
        update_agent_session(
            &pool,
            task_id1,
            s1.id,
            &UpdateAgentSessionRequest {
                status: AgentSessionStatus::Running,
            },
        )
        .await
        .expect("s1 to running");
        update_agent_session(
            &pool,
            task_id1,
            s1.id,
            &UpdateAgentSessionRequest {
                status: AgentSessionStatus::Completed,
            },
        )
        .await
        .expect("s1 to completed");

        // Create a failed session
        let s2 = create_agent_session(&pool, task_id2, &req)
            .await
            .expect("create s2");
        update_agent_session(
            &pool,
            task_id2,
            s2.id,
            &UpdateAgentSessionRequest {
                status: AgentSessionStatus::Running,
            },
        )
        .await
        .expect("s2 to running");
        update_agent_session(
            &pool,
            task_id2,
            s2.id,
            &UpdateAgentSessionRequest {
                status: AgentSessionStatus::Failed,
            },
        )
        .await
        .expect("s2 to failed");

        // Recover should not touch them
        let recovered = recover_orphaned_sessions(&pool)
            .await
            .expect("recover should succeed");
        assert_eq!(recovered.len(), 0, "terminal states should not be affected");

        // Verify statuses unchanged
        let r1 = get_agent_session(&pool, task_id1, s1.id)
            .await
            .expect("get s1");
        assert_eq!(r1.status, AgentSessionStatus::Completed);

        let r2 = get_agent_session(&pool, task_id2, s2.id)
            .await
            .expect("get s2");
        assert_eq!(r2.status, AgentSessionStatus::Failed);
    }
}
