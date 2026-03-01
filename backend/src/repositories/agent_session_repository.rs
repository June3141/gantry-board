//! Repository for agent_sessions table.

use chrono::{DateTime, Utc};
use sqlx::prelude::FromRow;
use sqlx::sqlite::SqliteConnection;
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::error::{AppError, AppResult};
use crate::models::agent_session::{AgentSession, AgentSessionStatus, AgentType};

#[derive(FromRow)]
struct AgentSessionRow {
    pub id: String,
    pub task_id: String,
    pub agent_type: AgentType,
    pub status: AgentSessionStatus,
    pub prompt: Option<String>,
    pub worktree_name: Option<String>,
    pub started_at: Option<DateTime<Utc>>,
    pub finished_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
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

fn row_to_agent_session(row: AgentSessionRow) -> AppResult<AgentSession> {
    row.try_into()
        .map_err(|e: uuid::Error| AppError::Internal(e.to_string()))
}

fn rows_to_agent_sessions(rows: Vec<AgentSessionRow>) -> AppResult<Vec<AgentSession>> {
    rows.into_iter().map(row_to_agent_session).collect()
}

pub async fn find_by_id(
    pool: &SqlitePool,
    task_id: Uuid,
    id: Uuid,
) -> AppResult<Option<AgentSession>> {
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

    row.map(row_to_agent_session).transpose()
}

pub async fn find_all_by_task(pool: &SqlitePool, task_id: Uuid) -> AppResult<Vec<AgentSession>> {
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

    rows_to_agent_sessions(rows)
}

/// Find sessions for a task that have a worktree and are in a terminal state.
pub async fn find_with_worktrees_terminal(
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

    rows_to_agent_sessions(rows)
}

/// Check that no active (pending/running/paused) session exists for a task, using a connection (for transactions).
pub async fn find_active_by_task_tx(
    conn: &mut SqliteConnection,
    task_id: Uuid,
) -> AppResult<Option<AgentSession>> {
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

    row.map(row_to_agent_session).transpose()
}

pub async fn insert(
    pool: &SqlitePool,
    id: Uuid,
    task_id: Uuid,
    agent_type: &AgentType,
    status: &AgentSessionStatus,
    now: DateTime<Utc>,
) -> AppResult<()> {
    sqlx::query(
        r#"
        INSERT INTO agent_sessions (id, task_id, agent_type, status, created_at, updated_at)
        VALUES ($1, $2, $3, $4, $5, $6)
        "#,
    )
    .bind(id.to_string())
    .bind(task_id.to_string())
    .bind(agent_type)
    .bind(status)
    .bind(now)
    .bind(now)
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn insert_tx(
    conn: &mut SqliteConnection,
    id: Uuid,
    task_id: Uuid,
    agent_type: &AgentType,
    status: &AgentSessionStatus,
    now: DateTime<Utc>,
) -> AppResult<()> {
    sqlx::query(
        r#"
        INSERT INTO agent_sessions (id, task_id, agent_type, status, created_at, updated_at)
        VALUES ($1, $2, $3, $4, $5, $6)
        "#,
    )
    .bind(id.to_string())
    .bind(task_id.to_string())
    .bind(agent_type)
    .bind(status)
    .bind(now)
    .bind(now)
    .execute(&mut *conn)
    .await?;

    Ok(())
}

pub async fn update_prompt(pool: &SqlitePool, session_id: Uuid, prompt: &str) -> AppResult<u64> {
    let result = sqlx::query(
        "UPDATE agent_sessions SET prompt = $1, updated_at = CURRENT_TIMESTAMP WHERE id = $2",
    )
    .bind(prompt)
    .bind(session_id.to_string())
    .execute(pool)
    .await?;

    Ok(result.rows_affected())
}

pub async fn update_prompt_tx(
    conn: &mut SqliteConnection,
    session_id: Uuid,
    prompt: &str,
) -> AppResult<u64> {
    let result = sqlx::query(
        "UPDATE agent_sessions SET prompt = $1, updated_at = CURRENT_TIMESTAMP WHERE id = $2",
    )
    .bind(prompt)
    .bind(session_id.to_string())
    .execute(&mut *conn)
    .await?;

    Ok(result.rows_affected())
}

pub async fn update_worktree_name(
    pool: &SqlitePool,
    session_id: Uuid,
    worktree_name: &str,
) -> AppResult<u64> {
    let result = sqlx::query(
        "UPDATE agent_sessions SET worktree_name = $1, updated_at = CURRENT_TIMESTAMP WHERE id = $2",
    )
    .bind(worktree_name)
    .bind(session_id.to_string())
    .execute(pool)
    .await?;

    Ok(result.rows_affected())
}

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

/// Mark all active sessions (running, paused, pending) as failed.
/// Returns info about the recovered sessions.
pub async fn recover_orphaned(pool: &SqlitePool) -> AppResult<Vec<RecoveredSession>> {
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

pub async fn update_status(
    pool: &SqlitePool,
    id: Uuid,
    task_id: Uuid,
    status: &AgentSessionStatus,
    started_at: Option<DateTime<Utc>>,
    finished_at: Option<DateTime<Utc>>,
    now: DateTime<Utc>,
) -> AppResult<()> {
    sqlx::query(
        r#"
        UPDATE agent_sessions
        SET status = $1, started_at = $2, finished_at = $3, updated_at = $4
        WHERE id = $5 AND task_id = $6
        "#,
    )
    .bind(status)
    .bind(started_at)
    .bind(finished_at)
    .bind(now)
    .bind(id.to_string())
    .bind(task_id.to_string())
    .execute(pool)
    .await?;

    Ok(())
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
                repository_path: None,
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

    async fn insert_test_session(pool: &SqlitePool, task_id: Uuid) -> Uuid {
        let id = Uuid::new_v4();
        let now = Utc::now();
        insert(
            pool,
            id,
            task_id,
            &AgentType::ClaudeCode,
            &AgentSessionStatus::Pending,
            now,
        )
        .await
        .expect("insert session");
        id
    }

    #[tokio::test]
    async fn test_insert_and_find_by_id() {
        let pool = setup_test_db().await;
        let (_project_id, task_id) = create_test_task(&pool).await;

        let id = insert_test_session(&pool, task_id).await;

        let session = find_by_id(&pool, task_id, id)
            .await
            .expect("find_by_id")
            .expect("should exist");

        assert_eq!(session.id, id);
        assert_eq!(session.task_id, task_id);
        assert!(matches!(session.agent_type, AgentType::ClaudeCode));
        assert_eq!(session.status, AgentSessionStatus::Pending);
        assert!(session.prompt.is_none());
        assert!(session.worktree_name.is_none());
        assert!(session.started_at.is_none());
        assert!(session.finished_at.is_none());
    }

    #[tokio::test]
    async fn test_find_by_id_returns_none_for_nonexistent() {
        let pool = setup_test_db().await;
        let (_project_id, task_id) = create_test_task(&pool).await;

        let result = find_by_id(&pool, task_id, Uuid::new_v4())
            .await
            .expect("find_by_id");

        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_find_all_by_task_empty() {
        let pool = setup_test_db().await;
        let (_project_id, task_id) = create_test_task(&pool).await;

        let sessions = find_all_by_task(&pool, task_id)
            .await
            .expect("find_all_by_task");

        assert!(sessions.is_empty());
    }

    #[tokio::test]
    async fn test_find_all_by_task_returns_sessions() {
        let pool = setup_test_db().await;
        let (_project_id, task_id) = create_test_task(&pool).await;

        // Insert first session, then mark it as completed so a second can be created
        let id1 = insert_test_session(&pool, task_id).await;
        let now = Utc::now();
        update_status(
            &pool,
            id1,
            task_id,
            &AgentSessionStatus::Running,
            Some(now),
            None,
            now,
        )
        .await
        .expect("to running");
        update_status(
            &pool,
            id1,
            task_id,
            &AgentSessionStatus::Completed,
            Some(now),
            Some(now),
            now,
        )
        .await
        .expect("to completed");

        let id2 = insert_test_session(&pool, task_id).await;

        let sessions = find_all_by_task(&pool, task_id)
            .await
            .expect("find_all_by_task");

        assert_eq!(sessions.len(), 2);
        let ids: Vec<Uuid> = sessions.iter().map(|s| s.id).collect();
        assert!(ids.contains(&id1));
        assert!(ids.contains(&id2));
    }

    #[tokio::test]
    async fn test_insert_tx_and_find() {
        let pool = setup_test_db().await;
        let (_project_id, task_id) = create_test_task(&pool).await;

        let id = Uuid::new_v4();
        let now = Utc::now();
        let mut tx = pool.begin().await.unwrap();
        insert_tx(
            &mut tx,
            id,
            task_id,
            &AgentType::GeminiCli,
            &AgentSessionStatus::Pending,
            now,
        )
        .await
        .expect("insert_tx");
        tx.commit().await.unwrap();

        let session = find_by_id(&pool, task_id, id)
            .await
            .expect("find_by_id")
            .expect("should exist");

        assert_eq!(session.id, id);
        assert!(matches!(session.agent_type, AgentType::GeminiCli));
    }

    #[tokio::test]
    async fn test_update_prompt() {
        let pool = setup_test_db().await;
        let (_project_id, task_id) = create_test_task(&pool).await;

        let id = insert_test_session(&pool, task_id).await;

        let affected = update_prompt(&pool, id, "test prompt")
            .await
            .expect("update_prompt");
        assert_eq!(affected, 1);

        let session = find_by_id(&pool, task_id, id)
            .await
            .expect("find_by_id")
            .expect("should exist");
        assert_eq!(session.prompt.as_deref(), Some("test prompt"));
    }

    #[tokio::test]
    async fn test_update_prompt_nonexistent_returns_zero() {
        let pool = setup_test_db().await;

        let affected = update_prompt(&pool, Uuid::new_v4(), "test")
            .await
            .expect("update_prompt");
        assert_eq!(affected, 0);
    }

    #[tokio::test]
    async fn test_update_prompt_tx() {
        let pool = setup_test_db().await;
        let (_project_id, task_id) = create_test_task(&pool).await;

        let id = insert_test_session(&pool, task_id).await;

        let mut tx = pool.begin().await.unwrap();
        let affected = update_prompt_tx(&mut tx, id, "tx prompt")
            .await
            .expect("update_prompt_tx");
        assert_eq!(affected, 1);
        tx.commit().await.unwrap();

        let session = find_by_id(&pool, task_id, id)
            .await
            .expect("find_by_id")
            .expect("should exist");
        assert_eq!(session.prompt.as_deref(), Some("tx prompt"));
    }

    #[tokio::test]
    async fn test_update_worktree_name() {
        let pool = setup_test_db().await;
        let (_project_id, task_id) = create_test_task(&pool).await;

        let id = insert_test_session(&pool, task_id).await;

        let affected = update_worktree_name(&pool, id, "my-worktree")
            .await
            .expect("update_worktree_name");
        assert_eq!(affected, 1);

        let session = find_by_id(&pool, task_id, id)
            .await
            .expect("find_by_id")
            .expect("should exist");
        assert_eq!(session.worktree_name.as_deref(), Some("my-worktree"));
    }

    #[tokio::test]
    async fn test_update_worktree_name_nonexistent_returns_zero() {
        let pool = setup_test_db().await;

        let affected = update_worktree_name(&pool, Uuid::new_v4(), "wt")
            .await
            .expect("update_worktree_name");
        assert_eq!(affected, 0);
    }

    #[tokio::test]
    async fn test_clear_worktree_name() {
        let pool = setup_test_db().await;
        let (_project_id, task_id) = create_test_task(&pool).await;

        let id = insert_test_session(&pool, task_id).await;
        update_worktree_name(&pool, id, "my-worktree")
            .await
            .expect("set worktree");

        clear_worktree_name(&pool, id)
            .await
            .expect("clear_worktree_name");

        let session = find_by_id(&pool, task_id, id)
            .await
            .expect("find_by_id")
            .expect("should exist");
        assert!(session.worktree_name.is_none());
    }

    #[tokio::test]
    async fn test_update_status() {
        let pool = setup_test_db().await;
        let (_project_id, task_id) = create_test_task(&pool).await;

        let id = insert_test_session(&pool, task_id).await;
        let now = Utc::now();

        update_status(
            &pool,
            id,
            task_id,
            &AgentSessionStatus::Running,
            Some(now),
            None,
            now,
        )
        .await
        .expect("update_status");

        let session = find_by_id(&pool, task_id, id)
            .await
            .expect("find_by_id")
            .expect("should exist");
        assert_eq!(session.status, AgentSessionStatus::Running);
        assert!(session.started_at.is_some());
        assert!(session.finished_at.is_none());
    }

    #[tokio::test]
    async fn test_find_active_by_task_tx() {
        let pool = setup_test_db().await;
        let (_project_id, task_id) = create_test_task(&pool).await;

        // No active session initially
        let mut tx = pool.begin().await.unwrap();
        let active = find_active_by_task_tx(&mut tx, task_id)
            .await
            .expect("find_active");
        assert!(active.is_none());
        tx.commit().await.unwrap();

        // Insert a pending session
        insert_test_session(&pool, task_id).await;

        let mut tx = pool.begin().await.unwrap();
        let active = find_active_by_task_tx(&mut tx, task_id)
            .await
            .expect("find_active");
        assert!(active.is_some());
        tx.commit().await.unwrap();
    }

    #[tokio::test]
    async fn test_find_with_worktrees_terminal() {
        let pool = setup_test_db().await;
        let (_project_id, task_id) = create_test_task(&pool).await;

        let id = insert_test_session(&pool, task_id).await;
        let now = Utc::now();

        // Set worktree name and move to completed
        update_worktree_name(&pool, id, "wt-1")
            .await
            .expect("set worktree");
        update_status(
            &pool,
            id,
            task_id,
            &AgentSessionStatus::Running,
            Some(now),
            None,
            now,
        )
        .await
        .expect("to running");
        update_status(
            &pool,
            id,
            task_id,
            &AgentSessionStatus::Completed,
            Some(now),
            Some(now),
            now,
        )
        .await
        .expect("to completed");

        let sessions = find_with_worktrees_terminal(&pool, task_id)
            .await
            .expect("find_with_worktrees_terminal");
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].id, id);
    }

    #[tokio::test]
    async fn test_find_with_worktrees_terminal_excludes_running() {
        let pool = setup_test_db().await;
        let (_project_id, task_id) = create_test_task(&pool).await;

        let id = insert_test_session(&pool, task_id).await;
        let now = Utc::now();

        // Set worktree name but keep running (non-terminal)
        update_worktree_name(&pool, id, "wt-1")
            .await
            .expect("set worktree");
        update_status(
            &pool,
            id,
            task_id,
            &AgentSessionStatus::Running,
            Some(now),
            None,
            now,
        )
        .await
        .expect("to running");

        let sessions = find_with_worktrees_terminal(&pool, task_id)
            .await
            .expect("find_with_worktrees_terminal");
        assert!(sessions.is_empty());
    }

    #[tokio::test]
    async fn test_recover_orphaned() {
        let pool = setup_test_db().await;
        let (_project_id, task_id) = create_test_task(&pool).await;

        let id = insert_test_session(&pool, task_id).await;
        let now = Utc::now();
        update_status(
            &pool,
            id,
            task_id,
            &AgentSessionStatus::Running,
            Some(now),
            None,
            now,
        )
        .await
        .expect("to running");

        let recovered = recover_orphaned(&pool).await.expect("recover");
        assert_eq!(recovered.len(), 1);

        let session = find_by_id(&pool, task_id, id)
            .await
            .expect("find_by_id")
            .expect("should exist");
        assert_eq!(session.status, AgentSessionStatus::Failed);
        assert!(session.finished_at.is_some());
    }

    #[tokio::test]
    async fn test_recover_orphaned_idempotent() {
        let pool = setup_test_db().await;
        let (_project_id, task_id) = create_test_task(&pool).await;

        let id = insert_test_session(&pool, task_id).await;
        let now = Utc::now();
        update_status(
            &pool,
            id,
            task_id,
            &AgentSessionStatus::Running,
            Some(now),
            None,
            now,
        )
        .await
        .expect("to running");

        let recovered1 = recover_orphaned(&pool).await.expect("first recover");
        assert_eq!(recovered1.len(), 1);

        let recovered2 = recover_orphaned(&pool).await.expect("second recover");
        assert_eq!(recovered2.len(), 0, "second call should be no-op");
    }
}
