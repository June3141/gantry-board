use std::convert::Infallible;
use std::pin::Pin;
use std::sync::atomic::Ordering;
use std::task::{Context, Poll};
use std::time::Duration;

use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::sse::{Event, KeepAlive, Sse};
use futures_util::stream::Stream;
use serde::Deserialize;
use tokio_stream::wrappers::BroadcastStream;
use tokio_stream::StreamExt;
use uuid::Uuid;

use crate::auth::middleware::AuthUser;
use crate::AppState;

/// Query parameters for the SSE endpoint.
#[derive(Debug, Deserialize)]
pub struct SseQuery {
    /// Optional project_id to filter events for a specific project.
    pub project_id: Option<Uuid>,
}

/// Wrapper stream that decrements the connection counter on drop.
struct TrackedStream<S> {
    inner: Pin<Box<S>>,
    state: AppState,
}

impl<S: Stream> Stream for TrackedStream<S> {
    type Item = S::Item;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.inner.as_mut().poll_next(cx)
    }
}

impl<S> Drop for TrackedStream<S> {
    fn drop(&mut self) {
        self.state.connection_counter.fetch_sub(1, Ordering::SeqCst);
        metrics::gauge!("gantry_sse_connections_active").decrement(1.0);
    }
}

/// SSE endpoint for real-time task updates.
///
/// Accepts an optional `project_id` query parameter. When provided, only
/// events matching that project are forwarded to the client. Events without
/// an embedded project_id are dropped for project-filtered subscribers.
pub async fn sse_handler(
    _auth: AuthUser,
    State(state): State<AppState>,
    Query(query): Query<SseQuery>,
) -> Result<Sse<impl Stream<Item = Result<Event, Infallible>>>, StatusCode> {
    // Check connection limit
    let max = state.config.max_realtime_connections;
    let current = state.connection_counter.fetch_add(1, Ordering::SeqCst);
    if current >= max {
        state.connection_counter.fetch_sub(1, Ordering::SeqCst);
        return Err(StatusCode::SERVICE_UNAVAILABLE);
    }

    metrics::gauge!("gantry_sse_connections_active").increment(1.0);

    let project_filter = query.project_id;

    let rx = state.sse_hub.subscribe();
    let stream = BroadcastStream::new(rx).filter_map(move |result| {
        result.ok().and_then(|event| {
            // Apply project filter if specified: only forward events
            // that belong to the subscribed project. Events without a
            // project_id are dropped to prevent cross-project leakage.
            if let Some(filter_pid) = project_filter {
                match event.project_id() {
                    Some(event_pid) if event_pid == filter_pid => {}
                    _ => return None,
                }
            }

            let event_type = event.event_type();
            serde_json::to_string(&event)
                .ok()
                .map(|data| Ok(Event::default().event(event_type).data(data)))
        })
    });

    Ok(Sse::new(TrackedStream {
        inner: Box::pin(stream),
        state: state.clone(),
    })
    .keep_alive(
        KeepAlive::new()
            .interval(Duration::from_secs(30))
            .text("keep-alive"),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::executor::NoopExecutor;
    use crate::agent::orchestrator::AgentOrchestrator;
    use crate::auth::middleware::AuthUser;
    use crate::config::Config;
    use crate::services::agent_session_output_service::OutputBuffer;
    use crate::sse::event::SseEvent;
    use crate::sse::hub::SseHub;
    use sqlx::sqlite::SqlitePoolOptions;
    use std::path::PathBuf;
    use std::sync::atomic::AtomicUsize;
    use std::sync::Arc;
    use uuid::Uuid;

    async fn create_test_state() -> AppState {
        let pool = SqlitePoolOptions::new()
            .connect("sqlite::memory:")
            .await
            .expect("Failed to create test pool");

        sqlx::migrate!("./migrations")
            .run(&pool)
            .await
            .expect("Failed to run migrations");

        let sse_hub = Arc::new(SseHub::default());
        let mut executors = std::collections::HashMap::new();
        executors.insert(
            crate::models::agent_session::AgentType::ClaudeCode,
            Arc::new(NoopExecutor) as Arc<dyn crate::agent::executor::AgentExecutor>,
        );
        let output_buffer = Arc::new(OutputBuffer::new(pool.clone()));
        let orchestrator = Arc::new(AgentOrchestrator::new(
            executors,
            pool.clone(),
            PathBuf::from("."),
            Arc::clone(&sse_hub),
            Arc::clone(&output_buffer),
        ));
        AppState {
            pool,
            sse_hub,
            config: Arc::new(Config::default()),
            orchestrator,
            preview_manager: None,
            github_client: None,
            output_buffer,
            connection_counter: Arc::new(AtomicUsize::new(0)),
            started_at: std::time::Instant::now(),
        }
    }

    #[tokio::test]
    async fn test_sse_handler_creates_stream() {
        let state = create_test_state().await;
        let auth = AuthUser {
            user_id: Uuid::new_v4(),
            session_id: Uuid::new_v4(),
        };

        // Just verify the handler can be called without panic
        let _sse = sse_handler(auth, State(state), Query(SseQuery { project_id: None })).await;
    }

    #[tokio::test]
    async fn test_sse_handler_rejects_when_connection_limit_reached() {
        let state = create_test_state().await;
        let state = AppState {
            config: Arc::new(Config {
                max_realtime_connections: 2,
                ..Default::default()
            }),
            ..state
        };

        // Simulate that 2 connections are already active
        state.connection_counter.store(2, Ordering::SeqCst);

        let auth = AuthUser {
            user_id: Uuid::new_v4(),
            session_id: Uuid::new_v4(),
        };

        let result = sse_handler(auth, State(state), Query(SseQuery { project_id: None })).await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_sse_handler_accepts_when_under_limit() {
        let state = create_test_state().await;

        let auth = AuthUser {
            user_id: Uuid::new_v4(),
            session_id: Uuid::new_v4(),
        };

        // Counter is 0, limit is 100 — should succeed
        let result = sse_handler(auth, State(state), Query(SseQuery { project_id: None })).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_sse_handler_increments_counter_on_connect() {
        let state = create_test_state().await;
        let counter = Arc::clone(&state.connection_counter);

        let auth = AuthUser {
            user_id: Uuid::new_v4(),
            session_id: Uuid::new_v4(),
        };

        assert_eq!(counter.load(Ordering::SeqCst), 0);

        let _sse = sse_handler(auth, State(state), Query(SseQuery { project_id: None }))
            .await
            .expect("should succeed");

        assert_eq!(counter.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn test_sse_receives_broadcast_event() {
        use crate::models::task::{Task, TaskPriority, TaskStatus};
        use chrono::Utc;
        use uuid::Uuid;

        let state = create_test_state().await;
        let sse_hub = state.sse_hub.clone();

        // Subscribe before broadcasting
        let mut rx = sse_hub.subscribe();

        let task = Task {
            id: Uuid::new_v4(),
            project_id: Uuid::new_v4(),
            title: "Test".to_string(),
            description: None,
            status: TaskStatus::Todo,
            priority: TaskPriority::Medium,
            parent_id: None,
            assigned_to: None,
            position: 0,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let task_id = task.id;
        sse_hub.broadcast(SseEvent::task_created(task));

        let event = rx.recv().await.expect("Should receive event");
        assert_eq!(event.event_type(), "task_created");

        if let SseEvent::TaskCreated { task } = event {
            assert_eq!(task.id, task_id);
        } else {
            panic!("Expected TaskCreated");
        }
    }
}
