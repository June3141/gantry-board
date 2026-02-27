use std::time::Duration;

use sqlx::SqlitePool;
use tokio::sync::Mutex;
use uuid::Uuid;

use crate::error::{AppError, AppResult};

use super::{MAX_CONTENT_SIZE, MAX_OUTPUTS_PER_SESSION};

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
    use crate::services::agent_session_output_service::get_outputs;
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
