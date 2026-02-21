use std::time::Duration;

use chrono::Utc;
use sqlx::SqlitePool;
use tokio::sync::Mutex;
use uuid::Uuid;

use crate::error::{AppError, AppResult};
use crate::models::agent_session_output::{AgentSessionOutput, AgentSessionOutputRow};

/// Maximum size of a single output content in bytes (64 KB).
const MAX_CONTENT_SIZE: usize = 64 * 1024;

/// Maximum number of output records per session.
const MAX_OUTPUTS_PER_SESSION: i64 = 10_000;

pub async fn append_output(
    pool: &SqlitePool,
    session_id: Uuid,
    sequence: i64,
    content: &str,
) -> AppResult<AgentSessionOutput> {
    if content.len() > MAX_CONTENT_SIZE {
        return Err(AppError::Validation(format!(
            "output content exceeds maximum size of {} bytes",
            MAX_CONTENT_SIZE
        )));
    }
    if sequence >= MAX_OUTPUTS_PER_SESSION {
        return Err(AppError::Validation(format!(
            "session output limit reached ({MAX_OUTPUTS_PER_SESSION} records)"
        )));
    }

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

pub async fn get_outputs_paginated(
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

/// Pending output waiting to be flushed.
struct PendingOutput {
    session_id: Uuid,
    sequence: i64,
    content: String,
}

/// Buffers agent outputs and flushes them in bulk to reduce DB write contention.
/// Flushes every `flush_interval` or when the buffer reaches `batch_size`,
/// or when total buffered bytes exceed `max_total_bytes`.
pub struct OutputBuffer {
    pool: SqlitePool,
    buffer: Mutex<Vec<PendingOutput>>,
    batch_size: usize,
    /// Maximum total buffered bytes before auto-flush (default: 10 MB).
    max_total_bytes: usize,
    /// Current total buffered bytes.
    total_bytes: Mutex<usize>,
}

const DEFAULT_BATCH_SIZE: usize = 100;
const DEFAULT_FLUSH_INTERVAL_MS: u64 = 500;
/// Default max total buffer size: 10 MB.
const DEFAULT_MAX_TOTAL_BYTES: usize = 10 * 1024 * 1024;

impl OutputBuffer {
    pub fn new(pool: SqlitePool) -> Self {
        Self {
            pool,
            buffer: Mutex::new(Vec::new()),
            batch_size: DEFAULT_BATCH_SIZE,
            max_total_bytes: DEFAULT_MAX_TOTAL_BYTES,
            total_bytes: Mutex::new(0),
        }
    }

    /// Create an OutputBuffer with custom limits (for testing).
    #[cfg(test)]
    pub fn with_limits(pool: SqlitePool, batch_size: usize, max_total_bytes: usize) -> Self {
        Self {
            pool,
            buffer: Mutex::new(Vec::new()),
            batch_size,
            max_total_bytes,
            total_bytes: Mutex::new(0),
        }
    }

    /// Add an output to the buffer. Flushes automatically when batch_size is reached
    /// or when total buffered bytes exceed `max_total_bytes`.
    pub async fn add(&self, session_id: Uuid, sequence: i64, content: String) -> AppResult<()> {
        if content.len() > MAX_CONTENT_SIZE {
            return Err(AppError::Validation(format!(
                "output content exceeds maximum size of {} bytes",
                MAX_CONTENT_SIZE
            )));
        }
        if sequence >= MAX_OUTPUTS_PER_SESSION {
            return Err(AppError::Validation(format!(
                "session output limit reached ({MAX_OUTPUTS_PER_SESSION} records)"
            )));
        }

        let content_len = content.len();
        let should_flush = {
            let mut buf = self.buffer.lock().await;
            let mut total = self.total_bytes.lock().await;
            buf.push(PendingOutput {
                session_id,
                sequence,
                content,
            });
            *total += content_len;
            buf.len() >= self.batch_size || *total >= self.max_total_bytes
        };

        if should_flush {
            self.flush().await?;
        }

        Ok(())
    }

    /// Flush all pending outputs to the database in a single transaction.
    pub async fn flush(&self) -> AppResult<()> {
        let items = {
            let mut buf = self.buffer.lock().await;
            let mut total = self.total_bytes.lock().await;
            *total = 0;
            std::mem::take(&mut *buf)
        };

        if items.is_empty() {
            return Ok(());
        }

        let mut tx = self.pool.begin().await?;
        for item in &items {
            sqlx::query(
                "INSERT INTO agent_session_outputs (session_id, sequence, content) VALUES ($1, $2, $3)",
            )
            .bind(item.session_id.to_string())
            .bind(item.sequence)
            .bind(&item.content)
            .execute(&mut *tx)
            .await?;
        }
        tx.commit().await?;

        tracing::debug!(count = items.len(), "flushed output buffer");
        Ok(())
    }

    /// Spawn a periodic flush task that runs until the token is cancelled.
    pub fn spawn_periodic_flush(
        self: &std::sync::Arc<Self>,
        cancel: tokio_util::sync::CancellationToken,
    ) {
        let buf = std::sync::Arc::clone(self);
        tokio::spawn(async move {
            let mut interval =
                tokio::time::interval(Duration::from_millis(DEFAULT_FLUSH_INTERVAL_MS));
            interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
            loop {
                tokio::select! {
                    _ = interval.tick() => {}
                    _ = cancel.cancelled() => {
                        // Final flush on shutdown
                        if let Err(e) = buf.flush().await {
                            tracing::warn!(error = %e, "final output buffer flush failed");
                        }
                        return;
                    }
                }
                if let Err(e) = buf.flush().await {
                    tracing::warn!(error = %e, "periodic output buffer flush failed");
                }
            }
        });
    }
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
    async fn test_append_output_rejects_oversized_content() {
        let pool = setup_test_db().await;
        let session_id = create_test_session(&pool).await;

        let big_content = "x".repeat(65 * 1024); // 65 KB > 64 KB limit
        let result = append_output(&pool, session_id, 0, &big_content).await;
        assert!(
            matches!(result, Err(AppError::Validation(_))),
            "oversized content should be rejected, got: {result:?}"
        );
    }

    #[tokio::test]
    async fn test_append_output_rejects_over_sequence_limit() {
        let pool = setup_test_db().await;
        let session_id = create_test_session(&pool).await;

        let result = append_output(&pool, session_id, 10_000, "content").await;
        assert!(
            matches!(result, Err(AppError::Validation(_))),
            "sequence >= limit should be rejected, got: {result:?}"
        );
    }

    #[tokio::test]
    async fn test_get_outputs_paginated_returns_limited_results() {
        let pool = setup_test_db().await;
        let session_id = create_test_session(&pool).await;

        // Insert 10 outputs
        for i in 0..10 {
            append_output(&pool, session_id, i, &format!("chunk-{i}"))
                .await
                .expect("Failed to append");
        }

        // Get first 5 outputs
        let outputs = get_outputs_paginated(&pool, session_id, 5, 0)
            .await
            .expect("Failed to get paginated");
        assert_eq!(outputs.len(), 5);
        assert_eq!(outputs[0].sequence, 0);
        assert_eq!(outputs[4].sequence, 4);
    }

    #[tokio::test]
    async fn test_get_outputs_paginated_with_offset() {
        let pool = setup_test_db().await;
        let session_id = create_test_session(&pool).await;

        // Insert 10 outputs
        for i in 0..10 {
            append_output(&pool, session_id, i, &format!("chunk-{i}"))
                .await
                .expect("Failed to append");
        }

        // Get outputs 5..10
        let outputs = get_outputs_paginated(&pool, session_id, 5, 5)
            .await
            .expect("Failed to get paginated");
        assert_eq!(outputs.len(), 5);
        assert_eq!(outputs[0].sequence, 5);
        assert_eq!(outputs[4].sequence, 9);
    }

    #[tokio::test]
    async fn test_get_outputs_paginated_offset_beyond_total() {
        let pool = setup_test_db().await;
        let session_id = create_test_session(&pool).await;

        for i in 0..3 {
            append_output(&pool, session_id, i, &format!("chunk-{i}"))
                .await
                .expect("Failed to append");
        }

        let outputs = get_outputs_paginated(&pool, session_id, 100, 100)
            .await
            .expect("Failed to get paginated");
        assert!(outputs.is_empty());
    }

    #[tokio::test]
    async fn test_get_outputs_paginated_default_limit() {
        let pool = setup_test_db().await;
        let session_id = create_test_session(&pool).await;

        for i in 0..5 {
            append_output(&pool, session_id, i, &format!("chunk-{i}"))
                .await
                .expect("Failed to append");
        }

        // Default limit (100) should return all 5
        let outputs = get_outputs_paginated(&pool, session_id, 100, 0)
            .await
            .expect("Failed to get paginated");
        assert_eq!(outputs.len(), 5);
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

    /// Issue #275: OutputBuffer should auto-flush when total buffered bytes exceed the cap.
    #[tokio::test]
    async fn test_output_buffer_flushes_when_total_size_exceeded() {
        let pool = setup_test_db().await;
        let session_id = create_test_session(&pool).await;

        let buf = OutputBuffer::with_limits(pool.clone(), 1000, 1024); // batch=1000 items, max_total_bytes=1 KB

        // Add content that exceeds the 1 KB total limit
        let chunk = "a".repeat(600); // 600 bytes each
        buf.add(session_id, 0, chunk.clone())
            .await
            .expect("first add should succeed");

        // Second add pushes total past 1 KB → should trigger flush
        buf.add(session_id, 1, chunk.clone())
            .await
            .expect("second add should succeed");

        // Verify that data has been flushed to DB
        let outputs = get_outputs(&pool, session_id).await.expect("get outputs");
        assert!(
            !outputs.is_empty(),
            "buffer should have flushed to DB when total size exceeded"
        );
    }

    /// Issue #275: OutputBuffer should not flush prematurely when under the cap.
    #[tokio::test]
    async fn test_output_buffer_does_not_flush_under_cap() {
        let pool = setup_test_db().await;
        let session_id = create_test_session(&pool).await;

        // batch=1000 items, max_total_bytes=10 MB (effectively unlimited for this test)
        let buf = OutputBuffer::with_limits(pool.clone(), 1000, 10 * 1024 * 1024);

        let chunk = "a".repeat(100); // 100 bytes — well under the cap
        buf.add(session_id, 0, chunk)
            .await
            .expect("add should succeed");

        // Data should still be buffered, not flushed yet
        let outputs = get_outputs(&pool, session_id).await.expect("get outputs");
        assert!(
            outputs.is_empty(),
            "buffer should NOT have flushed yet (under size cap and batch limit)"
        );

        // Explicit flush should persist it
        buf.flush().await.expect("flush should succeed");
        let outputs = get_outputs(&pool, session_id).await.expect("get outputs");
        assert_eq!(outputs.len(), 1);
    }
}
