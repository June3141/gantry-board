use std::time::Duration;

use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::State;
use axum::response::IntoResponse;
use tokio::sync::broadcast;

use super::event::SseEvent;
use crate::auth::middleware::AuthUser;
use crate::AppState;

/// WebSocket endpoint for real-time updates (alternative to SSE).
///
/// Subscribes to the same broadcast hub as the SSE handler and forwards
/// events as JSON text frames. Clients that cannot use SSE (e.g. behind
/// certain proxies) can connect here instead.
pub async fn ws_handler(
    _auth: AuthUser,
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let rx = state.sse_hub.subscribe();
    ws.on_upgrade(move |socket| handle_socket(socket, rx))
}

/// Convert an SseEvent into a WebSocket JSON text message.
///
/// Format: `{"event": "<event_type>", "data": <serialized_event>}`
/// The `data` field matches the JSON payload sent via SSE `data:` lines,
/// allowing the frontend to parse both transports identically.
fn event_to_ws_message(event: &SseEvent) -> Option<Message> {
    let data = serde_json::to_value(event).ok()?;
    let msg = serde_json::json!({
        "event": event.event_type(),
        "data": data,
    });
    Some(Message::Text(msg.to_string().into()))
}

async fn handle_socket(mut socket: WebSocket, mut rx: broadcast::Receiver<SseEvent>) {
    metrics::gauge!("gantry_ws_connections_active").increment(1.0);

    let mut ping_interval = tokio::time::interval(Duration::from_secs(30));
    ping_interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);

    loop {
        tokio::select! {
            event = rx.recv() => {
                match event {
                    Ok(event) => {
                        if let Some(msg) = event_to_ws_message(&event) {
                            if socket.send(msg).await.is_err() {
                                break;
                            }
                        }
                    }
                    Err(broadcast::error::RecvError::Lagged(n)) => {
                        tracing::warn!(skipped = n, "WebSocket client lagged");
                    }
                    Err(broadcast::error::RecvError::Closed) => break,
                }
            }
            _ = ping_interval.tick() => {
                if socket.send(Message::Ping(vec![].into())).await.is_err() {
                    break;
                }
            }
            msg = socket.recv() => {
                match msg {
                    Some(Ok(Message::Close(_))) | None => break,
                    Some(Ok(_)) => {}
                    Some(Err(_)) => break,
                }
            }
        }
    }

    metrics::gauge!("gantry_ws_connections_active").decrement(1.0);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::task::{Task, TaskPriority, TaskStatus};
    use chrono::Utc;
    use uuid::Uuid;

    fn create_test_task() -> Task {
        Task {
            id: Uuid::new_v4(),
            project_id: Uuid::new_v4(),
            title: "Test Task".to_string(),
            description: None,
            status: TaskStatus::Todo,
            priority: TaskPriority::Medium,
            parent_id: None,
            assigned_to: None,
            position: 0,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    #[test]
    fn test_event_to_ws_message_task_created() {
        let task = create_test_task();
        let task_id = task.id;
        let event = SseEvent::task_created(task);

        let msg = event_to_ws_message(&event).expect("should serialize");

        if let Message::Text(text) = msg {
            let parsed: serde_json::Value =
                serde_json::from_str(&text).expect("should be valid JSON");
            assert_eq!(parsed["event"], "task_created");
            assert_eq!(parsed["data"]["type"], "TaskCreated");
            assert_eq!(parsed["data"]["task"]["id"], task_id.to_string());
        } else {
            panic!("Expected Text message");
        }
    }

    #[test]
    fn test_event_to_ws_message_agent_output() {
        let session_id = Uuid::new_v4();
        let event = SseEvent::agent_output(session_id, "Hello from agent".to_string());

        let msg = event_to_ws_message(&event).expect("should serialize");

        if let Message::Text(text) = msg {
            let parsed: serde_json::Value =
                serde_json::from_str(&text).expect("should be valid JSON");
            assert_eq!(parsed["event"], "agent_output");
            assert_eq!(parsed["data"]["session_id"], session_id.to_string());
            assert_eq!(parsed["data"]["text"], "Hello from agent");
        } else {
            panic!("Expected Text message");
        }
    }

    #[test]
    fn test_event_to_ws_message_task_deleted() {
        let task_id = Uuid::new_v4();
        let event = SseEvent::task_deleted(task_id);

        let msg = event_to_ws_message(&event).expect("should serialize");

        if let Message::Text(text) = msg {
            let parsed: serde_json::Value =
                serde_json::from_str(&text).expect("should be valid JSON");
            assert_eq!(parsed["event"], "task_deleted");
            assert_eq!(parsed["data"]["task_id"], task_id.to_string());
        } else {
            panic!("Expected Text message");
        }
    }

    #[test]
    fn test_event_to_ws_message_all_event_types() {
        let events = vec![
            SseEvent::task_created(create_test_task()),
            SseEvent::task_updated(create_test_task()),
            SseEvent::task_deleted(Uuid::new_v4()),
            SseEvent::agent_output(Uuid::new_v4(), "test".to_string()),
            SseEvent::comment_deleted(Uuid::new_v4(), Uuid::new_v4()),
            SseEvent::preview_deleted(Uuid::new_v4()),
            SseEvent::github_sync_failed(Uuid::new_v4(), "error".to_string()),
        ];

        for event in &events {
            let msg = event_to_ws_message(event);
            assert!(
                msg.is_some(),
                "Failed to serialize event type: {}",
                event.event_type()
            );
        }
    }

    #[test]
    fn test_event_to_ws_message_json_structure() {
        let task = create_test_task();
        let event = SseEvent::task_created(task);

        let msg = event_to_ws_message(&event).expect("should serialize");

        if let Message::Text(text) = msg {
            let parsed: serde_json::Value = serde_json::from_str(&text).unwrap();
            let obj = parsed.as_object().expect("should be an object");
            assert!(obj.contains_key("event"), "missing 'event' key");
            assert!(obj.contains_key("data"), "missing 'data' key");
            assert_eq!(obj.len(), 2, "should have exactly 2 keys");
        } else {
            panic!("Expected Text message");
        }
    }
}
