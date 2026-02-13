use std::convert::Infallible;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::Duration;

use axum::extract::State;
use axum::response::sse::{Event, KeepAlive, Sse};
use futures_util::stream::Stream;
use tokio_stream::wrappers::BroadcastStream;
use tokio_stream::StreamExt;

use crate::auth::middleware::AuthUser;
use crate::AppState;

/// Wrapper stream that decrements the SSE connection gauge on drop.
struct TrackedStream<S> {
    inner: Pin<Box<S>>,
}

impl<S: Stream> Stream for TrackedStream<S> {
    type Item = S::Item;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.inner.as_mut().poll_next(cx)
    }
}

impl<S> Drop for TrackedStream<S> {
    fn drop(&mut self) {
        metrics::gauge!("gantry_sse_connections_active").decrement(1.0);
    }
}

/// SSE endpoint for real-time task updates
pub async fn sse_handler(
    _auth: AuthUser,
    State(state): State<AppState>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    metrics::gauge!("gantry_sse_connections_active").increment(1.0);

    let rx = state.sse_hub.subscribe();
    let stream = BroadcastStream::new(rx).filter_map(|result| {
        result.ok().and_then(|event| {
            let event_type = event.event_type();
            serde_json::to_string(&event)
                .ok()
                .map(|data| Ok(Event::default().event(event_type).data(data)))
        })
    });

    Sse::new(TrackedStream {
        inner: Box::pin(stream),
    })
    .keep_alive(
        KeepAlive::new()
            .interval(Duration::from_secs(30))
            .text("keep-alive"),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::executor::NoopExecutor;
    use crate::agent::orchestrator::AgentOrchestrator;
    use crate::auth::middleware::AuthUser;
    use crate::config::Config;
    use crate::sse::event::SseEvent;
    use crate::sse::hub::SseHub;
    use sqlx::sqlite::SqlitePoolOptions;
    use std::path::PathBuf;
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
        let orchestrator = Arc::new(AgentOrchestrator::new(
            executors,
            pool.clone(),
            PathBuf::from("."),
            Arc::clone(&sse_hub),
        ));
        AppState {
            pool,
            sse_hub,
            config: Arc::new(Config::default()),
            orchestrator,
            preview_manager: None,
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
        let _sse = sse_handler(auth, State(state)).await;
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
