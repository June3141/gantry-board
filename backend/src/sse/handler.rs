use std::convert::Infallible;
use std::time::Duration;

use axum::extract::State;
use axum::response::sse::{Event, KeepAlive, Sse};
use futures_util::stream::Stream;
use tokio_stream::wrappers::BroadcastStream;
use tokio_stream::StreamExt;

use crate::AppState;

/// SSE endpoint for real-time task updates
pub async fn sse_handler(
    State(state): State<AppState>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let rx = state.sse_hub.subscribe();
    let stream = BroadcastStream::new(rx).filter_map(|result| {
        result.ok().and_then(|event| {
            let event_type = event.event_type();
            serde_json::to_string(&event)
                .ok()
                .map(|data| Ok(Event::default().event(event_type).data(data)))
        })
    });

    Sse::new(stream).keep_alive(
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
    use crate::config::Config;
    use crate::sse::event::SseEvent;
    use crate::sse::hub::SseHub;
    use sqlx::sqlite::SqlitePoolOptions;
    use std::path::PathBuf;
    use std::sync::Arc;

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
        let orchestrator = Arc::new(AgentOrchestrator::new(
            Arc::new(NoopExecutor),
            pool.clone(),
            PathBuf::from("."),
            Arc::clone(&sse_hub),
        ));
        AppState {
            pool,
            sse_hub,
            config: Arc::new(Config::default()),
            orchestrator,
        }
    }

    #[tokio::test]
    async fn test_sse_handler_creates_stream() {
        let state = create_test_state().await;

        // Just verify the handler can be called without panic
        let _sse = sse_handler(State(state)).await;
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
