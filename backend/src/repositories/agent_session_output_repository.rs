//! Repository for agent_session_outputs table.

use chrono::{DateTime, Utc};
use sqlx::prelude::FromRow;
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::error::{AppError, AppResult};
use crate::models::agent_session_output::AgentSessionOutput;

#[derive(FromRow)]
struct AgentSessionOutputRow {
    pub id: i64,
    pub session_id: String,
    pub sequence: i64,
    pub content: String,
    pub created_at: DateTime<Utc>,
}

impl TryFrom<AgentSessionOutputRow> for AgentSessionOutput {
    type Error = uuid::Error;

    fn try_from(row: AgentSessionOutputRow) -> Result<Self, Self::Error> {
        Ok(AgentSessionOutput {
            id: row.id,
            session_id: row.session_id.parse()?,
            sequence: row.sequence,
            content: row.content,
            created_at: row.created_at,
        })
    }
}

fn row_to_output(row: AgentSessionOutputRow) -> AppResult<AgentSessionOutput> {
    row.try_into()
        .map_err(|e: uuid::Error| AppError::Internal(e.to_string()))
}

fn rows_to_outputs(rows: Vec<AgentSessionOutputRow>) -> AppResult<Vec<AgentSessionOutput>> {
    rows.into_iter().map(row_to_output).collect()
}

pub async fn insert_returning(
    pool: &SqlitePool,
    session_id: Uuid,
    sequence: i64,
    content: &str,
) -> AppResult<AgentSessionOutput> {
    let row = sqlx::query_as::<_, AgentSessionOutputRow>(
        r#"
        INSERT INTO agent_session_outputs (session_id, sequence, content)
        VALUES ($1, $2, $3)
        RETURNING id, session_id, sequence, content, created_at
        "#,
    )
    .bind(session_id.to_string())
    .bind(sequence)
    .bind(content)
    .fetch_one(pool)
    .await?;

    row_to_output(row)
}

pub async fn find_all_by_session(
    pool: &SqlitePool,
    session_id: Uuid,
) -> AppResult<Vec<AgentSessionOutput>> {
    let rows = sqlx::query_as::<_, AgentSessionOutputRow>(
        r#"
        SELECT id, session_id, sequence, content, created_at
        FROM agent_session_outputs
        WHERE session_id = $1
        ORDER BY sequence ASC
        "#,
    )
    .bind(session_id.to_string())
    .fetch_all(pool)
    .await?;

    rows_to_outputs(rows)
}

pub async fn find_by_session_paginated(
    pool: &SqlitePool,
    session_id: Uuid,
    limit: i64,
    offset: i64,
) -> AppResult<Vec<AgentSessionOutput>> {
    let rows = sqlx::query_as::<_, AgentSessionOutputRow>(
        r#"
        SELECT id, session_id, sequence, content, created_at
        FROM agent_session_outputs
        WHERE session_id = $1
        ORDER BY sequence ASC
        LIMIT $2 OFFSET $3
        "#,
    )
    .bind(session_id.to_string())
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await?;

    rows_to_outputs(rows)
}

pub async fn find_by_session_after_sequence(
    pool: &SqlitePool,
    session_id: Uuid,
    after_sequence: i64,
) -> AppResult<Vec<AgentSessionOutput>> {
    let rows = sqlx::query_as::<_, AgentSessionOutputRow>(
        r#"
        SELECT id, session_id, sequence, content, created_at
        FROM agent_session_outputs
        WHERE session_id = $1 AND sequence > $2
        ORDER BY sequence ASC
        "#,
    )
    .bind(session_id.to_string())
    .bind(after_sequence)
    .fetch_all(pool)
    .await?;

    rows_to_outputs(rows)
}

/// Delete outputs older than the given cutoff date for completed/failed/cancelled sessions.
pub async fn delete_old_for_terminal_sessions(
    pool: &SqlitePool,
    cutoff: DateTime<Utc>,
) -> AppResult<u64> {
    let result = sqlx::query(
        r#"
        DELETE FROM agent_session_outputs
        WHERE created_at <= $1
          AND session_id IN (
            SELECT id FROM agent_sessions
            WHERE status IN ('completed', 'failed', 'cancelled')
          )
        "#,
    )
    .bind(cutoff.format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string())
    .execute(pool)
    .await?;

    Ok(result.rows_affected())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::agent_session::{AgentSessionStatus, AgentType, CreateAgentSessionRequest};
    use crate::models::project::CreateProjectRequest;
    use crate::models::task::CreateTaskRequest;
    use crate::services::{agent_session_service, project_service, task_service};
    use crate::test_helpers::setup_test_db;

    async fn create_test_session(pool: &SqlitePool) -> Uuid {
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

        let session = agent_session_service::create_agent_session(
            pool,
            task.id,
            &CreateAgentSessionRequest {
                agent_type: AgentType::ClaudeCode,
            },
        )
        .await
        .expect("Failed to create session");

        session.id
    }

    #[tokio::test]
    async fn test_insert_returning_and_find_all() {
        let pool = setup_test_db().await;
        let session_id = create_test_session(&pool).await;

        let out1 = insert_returning(&pool, session_id, 0, "Hello ")
            .await
            .expect("insert 1");
        let out2 = insert_returning(&pool, session_id, 1, "World")
            .await
            .expect("insert 2");

        assert_eq!(out1.sequence, 0);
        assert_eq!(out1.content, "Hello ");
        assert_eq!(out1.session_id, session_id);
        assert_eq!(out2.sequence, 1);
        assert_eq!(out2.content, "World");

        let outputs = find_all_by_session(&pool, session_id)
            .await
            .expect("find_all");
        assert_eq!(outputs.len(), 2);
        assert_eq!(outputs[0].sequence, 0);
        assert_eq!(outputs[1].sequence, 1);
    }

    #[tokio::test]
    async fn test_find_all_by_session_empty() {
        let pool = setup_test_db().await;
        let session_id = create_test_session(&pool).await;

        let outputs = find_all_by_session(&pool, session_id)
            .await
            .expect("find_all");
        assert!(outputs.is_empty());
    }

    #[tokio::test]
    async fn test_find_all_by_session_ordered_by_sequence() {
        let pool = setup_test_db().await;
        let session_id = create_test_session(&pool).await;

        insert_returning(&pool, session_id, 2, "Third")
            .await
            .expect("insert");
        insert_returning(&pool, session_id, 0, "First")
            .await
            .expect("insert");
        insert_returning(&pool, session_id, 1, "Second")
            .await
            .expect("insert");

        let outputs = find_all_by_session(&pool, session_id)
            .await
            .expect("find_all");
        assert_eq!(outputs.len(), 3);
        assert_eq!(outputs[0].content, "First");
        assert_eq!(outputs[1].content, "Second");
        assert_eq!(outputs[2].content, "Third");
    }

    #[tokio::test]
    async fn test_find_by_session_after_sequence() {
        let pool = setup_test_db().await;
        let session_id = create_test_session(&pool).await;

        for i in 0..5 {
            insert_returning(&pool, session_id, i, &format!("chunk-{i}"))
                .await
                .expect("insert");
        }

        let outputs = find_by_session_after_sequence(&pool, session_id, 2)
            .await
            .expect("find_after");
        assert_eq!(outputs.len(), 2);
        assert_eq!(outputs[0].sequence, 3);
        assert_eq!(outputs[1].sequence, 4);
    }

    #[tokio::test]
    async fn test_find_by_session_paginated() {
        let pool = setup_test_db().await;
        let session_id = create_test_session(&pool).await;

        for i in 0..10 {
            insert_returning(&pool, session_id, i, &format!("chunk-{i}"))
                .await
                .expect("insert");
        }

        let page1 = find_by_session_paginated(&pool, session_id, 5, 0)
            .await
            .expect("page 1");
        assert_eq!(page1.len(), 5);
        assert_eq!(page1[0].sequence, 0);
        assert_eq!(page1[4].sequence, 4);

        let page2 = find_by_session_paginated(&pool, session_id, 5, 5)
            .await
            .expect("page 2");
        assert_eq!(page2.len(), 5);
        assert_eq!(page2[0].sequence, 5);
        assert_eq!(page2[4].sequence, 9);
    }

    #[tokio::test]
    async fn test_find_by_session_paginated_offset_beyond_total() {
        let pool = setup_test_db().await;
        let session_id = create_test_session(&pool).await;

        for i in 0..3 {
            insert_returning(&pool, session_id, i, &format!("chunk-{i}"))
                .await
                .expect("insert");
        }

        let outputs = find_by_session_paginated(&pool, session_id, 100, 100)
            .await
            .expect("paginated");
        assert!(outputs.is_empty());
    }

    #[tokio::test]
    async fn test_duplicate_sequence_returns_error() {
        let pool = setup_test_db().await;
        let session_id = create_test_session(&pool).await;

        insert_returning(&pool, session_id, 0, "first")
            .await
            .expect("insert first");

        let result = insert_returning(&pool, session_id, 0, "duplicate").await;
        assert!(
            result.is_err(),
            "duplicate sequence should fail, got: {result:?}"
        );
    }

    #[tokio::test]
    async fn test_delete_old_for_terminal_sessions() {
        let pool = setup_test_db().await;
        let session_id = create_test_session(&pool).await;

        insert_returning(&pool, session_id, 0, "old output")
            .await
            .expect("insert");

        // Mark session as completed
        let task_id = get_task_id_for_session(&pool, session_id).await;
        agent_session_service::update_agent_session(
            &pool,
            task_id,
            session_id,
            &crate::models::agent_session::UpdateAgentSessionRequest {
                status: AgentSessionStatus::Running,
            },
        )
        .await
        .expect("to running");
        agent_session_service::update_agent_session(
            &pool,
            task_id,
            session_id,
            &crate::models::agent_session::UpdateAgentSessionRequest {
                status: AgentSessionStatus::Completed,
            },
        )
        .await
        .expect("to completed");

        // Backdate the output
        sqlx::query(
            "UPDATE agent_session_outputs SET created_at = '2020-01-01T00:00:00.000Z' WHERE session_id = $1",
        )
        .bind(session_id.to_string())
        .execute(&pool)
        .await
        .expect("backdate");

        let cutoff = Utc::now() - chrono::Duration::days(30);
        let count = delete_old_for_terminal_sessions(&pool, cutoff)
            .await
            .expect("delete_old");
        assert_eq!(count, 1);

        let outputs = find_all_by_session(&pool, session_id)
            .await
            .expect("find_all");
        assert!(outputs.is_empty());
    }

    #[tokio::test]
    async fn test_delete_old_preserves_recent() {
        let pool = setup_test_db().await;
        let session_id = create_test_session(&pool).await;

        insert_returning(&pool, session_id, 0, "recent output")
            .await
            .expect("insert");

        // Mark session as completed
        let task_id = get_task_id_for_session(&pool, session_id).await;
        agent_session_service::update_agent_session(
            &pool,
            task_id,
            session_id,
            &crate::models::agent_session::UpdateAgentSessionRequest {
                status: AgentSessionStatus::Running,
            },
        )
        .await
        .expect("to running");
        agent_session_service::update_agent_session(
            &pool,
            task_id,
            session_id,
            &crate::models::agent_session::UpdateAgentSessionRequest {
                status: AgentSessionStatus::Completed,
            },
        )
        .await
        .expect("to completed");

        let cutoff = Utc::now() - chrono::Duration::days(30);
        let count = delete_old_for_terminal_sessions(&pool, cutoff)
            .await
            .expect("delete_old");
        assert_eq!(count, 0);

        let outputs = find_all_by_session(&pool, session_id)
            .await
            .expect("find_all");
        assert_eq!(outputs.len(), 1);
    }

    /// Helper to get task_id from a session_id (for tests only)
    async fn get_task_id_for_session(pool: &SqlitePool, session_id: Uuid) -> Uuid {
        let row: (String,) = sqlx::query_as("SELECT task_id FROM agent_sessions WHERE id = $1")
            .bind(session_id.to_string())
            .fetch_one(pool)
            .await
            .expect("Failed to get task_id");
        row.0.parse().expect("Failed to parse task_id")
    }
}
