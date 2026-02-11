use chrono::Utc;
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::error::{AppError, AppResult};
use crate::models::agent_session_output::{AgentSessionOutput, AgentSessionOutputRow};

pub async fn append_output(
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

    row.try_into()
        .map_err(|e: uuid::Error| AppError::Internal(e.to_string()))
}

pub async fn get_outputs(
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

    rows.into_iter()
        .map(|r| r.try_into())
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e: uuid::Error| AppError::Internal(e.to_string()))
}

pub async fn get_outputs_after(
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

    rows.into_iter()
        .map(|r| r.try_into())
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e: uuid::Error| AppError::Internal(e.to_string()))
}

/// Delete agent session outputs older than the given number of days.
/// Only deletes outputs for completed/failed/cancelled sessions.
pub async fn cleanup_old_outputs(pool: &SqlitePool, retention_days: u64) -> AppResult<u64> {
    let retention_days_i64 = i64::try_from(retention_days)
        .map_err(|_| AppError::Internal("retention_days too large".to_string()))?;
    let cutoff = Utc::now() - chrono::Duration::days(retention_days_i64);
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
    use crate::models::agent_session::{AgentType, CreateAgentSessionRequest};
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
    async fn test_append_and_get_outputs_roundtrip() {
        let pool = setup_test_db().await;
        let session_id = create_test_session(&pool).await;

        let out1 = append_output(&pool, session_id, 0, "Hello ")
            .await
            .expect("Failed to append");
        let out2 = append_output(&pool, session_id, 1, "World")
            .await
            .expect("Failed to append");

        assert_eq!(out1.sequence, 0);
        assert_eq!(out1.content, "Hello ");
        assert_eq!(out1.session_id, session_id);
        assert_eq!(out2.sequence, 1);
        assert_eq!(out2.content, "World");

        let outputs = get_outputs(&pool, session_id).await.expect("Failed to get");
        assert_eq!(outputs.len(), 2);
        assert_eq!(outputs[0].sequence, 0);
        assert_eq!(outputs[0].content, "Hello ");
        assert_eq!(outputs[1].sequence, 1);
        assert_eq!(outputs[1].content, "World");
    }

    #[tokio::test]
    async fn test_get_outputs_returns_ordered_by_sequence() {
        let pool = setup_test_db().await;
        let session_id = create_test_session(&pool).await;

        append_output(&pool, session_id, 2, "Third")
            .await
            .expect("Failed");
        append_output(&pool, session_id, 0, "First")
            .await
            .expect("Failed");
        append_output(&pool, session_id, 1, "Second")
            .await
            .expect("Failed");

        let outputs = get_outputs(&pool, session_id).await.expect("Failed to get");
        assert_eq!(outputs.len(), 3);
        assert_eq!(outputs[0].content, "First");
        assert_eq!(outputs[1].content, "Second");
        assert_eq!(outputs[2].content, "Third");
    }

    #[tokio::test]
    async fn test_get_outputs_empty_session() {
        let pool = setup_test_db().await;
        let session_id = create_test_session(&pool).await;

        let outputs = get_outputs(&pool, session_id).await.expect("Failed to get");
        assert!(outputs.is_empty());
    }

    #[tokio::test]
    async fn test_get_outputs_after_sequence() {
        let pool = setup_test_db().await;
        let session_id = create_test_session(&pool).await;

        for i in 0..5 {
            append_output(&pool, session_id, i, &format!("chunk-{}", i))
                .await
                .expect("Failed");
        }

        let outputs = get_outputs_after(&pool, session_id, 2)
            .await
            .expect("Failed to get");
        assert_eq!(outputs.len(), 2);
        assert_eq!(outputs[0].sequence, 3);
        assert_eq!(outputs[1].sequence, 4);
    }

    #[tokio::test]
    async fn test_cleanup_old_outputs_deletes_expired() {
        let pool = setup_test_db().await;
        let session_id = create_test_session(&pool).await;

        // Add output
        append_output(&pool, session_id, 0, "old output")
            .await
            .expect("Failed to append");

        // Mark session as completed
        agent_session_service::update_agent_session(
            &pool,
            get_task_id_for_session(&pool, session_id).await,
            session_id,
            &crate::models::agent_session::UpdateAgentSessionRequest {
                status: crate::models::agent_session::AgentSessionStatus::Running,
            },
        )
        .await
        .expect("Failed to update to running");
        agent_session_service::update_agent_session(
            &pool,
            get_task_id_for_session(&pool, session_id).await,
            session_id,
            &crate::models::agent_session::UpdateAgentSessionRequest {
                status: crate::models::agent_session::AgentSessionStatus::Completed,
            },
        )
        .await
        .expect("Failed to update to completed");

        // Backdate the output's created_at
        sqlx::query("UPDATE agent_session_outputs SET created_at = '2020-01-01T00:00:00.000Z' WHERE session_id = $1")
            .bind(session_id.to_string())
            .execute(&pool)
            .await
            .expect("Failed to backdate");

        // Cleanup with 30-day retention — should delete the backdated output
        let count = cleanup_old_outputs(&pool, 30)
            .await
            .expect("Failed to cleanup");
        assert_eq!(count, 1);

        let outputs = get_outputs(&pool, session_id).await.expect("Failed to get");
        assert!(outputs.is_empty());
    }

    #[tokio::test]
    async fn test_cleanup_old_outputs_preserves_recent() {
        let pool = setup_test_db().await;
        let session_id = create_test_session(&pool).await;

        // Add output (created_at is now, so it's recent)
        append_output(&pool, session_id, 0, "recent output")
            .await
            .expect("Failed to append");

        // Mark session as completed
        agent_session_service::update_agent_session(
            &pool,
            get_task_id_for_session(&pool, session_id).await,
            session_id,
            &crate::models::agent_session::UpdateAgentSessionRequest {
                status: crate::models::agent_session::AgentSessionStatus::Running,
            },
        )
        .await
        .expect("Failed to update to running");
        agent_session_service::update_agent_session(
            &pool,
            get_task_id_for_session(&pool, session_id).await,
            session_id,
            &crate::models::agent_session::UpdateAgentSessionRequest {
                status: crate::models::agent_session::AgentSessionStatus::Completed,
            },
        )
        .await
        .expect("Failed to update to completed");

        // Cleanup with 30-day retention — recent output should be preserved
        let count = cleanup_old_outputs(&pool, 30)
            .await
            .expect("Failed to cleanup");
        assert_eq!(count, 0);

        let outputs = get_outputs(&pool, session_id).await.expect("Failed to get");
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

    #[tokio::test]
    async fn test_duplicate_sequence_returns_error() {
        let pool = setup_test_db().await;
        let session_id = create_test_session(&pool).await;

        append_output(&pool, session_id, 0, "first")
            .await
            .expect("First insert should succeed");

        let result = append_output(&pool, session_id, 0, "duplicate").await;
        assert!(
            matches!(result, Err(AppError::Database(_))),
            "Duplicate sequence should fail with database error, got: {result:?}"
        );
    }
}
