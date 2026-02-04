use tokio::sync::broadcast;

use super::event::SseEvent;

#[derive(Debug, Clone)]
pub struct SseHub {
    sender: broadcast::Sender<SseEvent>,
}

impl SseHub {
    pub fn new(capacity: usize) -> Self {
        let (sender, _) = broadcast::channel(capacity);
        Self { sender }
    }

    pub fn broadcast(&self, event: SseEvent) {
        // Ignore error when no receivers are connected
        let _ = self.sender.send(event);
    }

    pub fn subscribe(&self) -> broadcast::Receiver<SseEvent> {
        self.sender.subscribe()
    }
}

impl Default for SseHub {
    fn default() -> Self {
        Self::new(256)
    }
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

    #[tokio::test]
    async fn test_hub_broadcast_with_subscriber() {
        let hub = SseHub::new(16);
        let mut rx = hub.subscribe();

        let task = create_test_task();
        let task_id = task.id;
        hub.broadcast(SseEvent::task_created(task));

        let received = rx.recv().await.expect("Should receive event");
        if let SseEvent::TaskCreated { task } = received {
            assert_eq!(task.id, task_id);
        } else {
            panic!("Expected TaskCreated event");
        }
    }

    #[tokio::test]
    async fn test_hub_broadcast_without_subscriber() {
        let hub = SseHub::new(16);
        let task = create_test_task();

        // Should not panic when no subscribers
        hub.broadcast(SseEvent::task_created(task));
    }

    #[tokio::test]
    async fn test_hub_multiple_subscribers() {
        let hub = SseHub::new(16);
        let mut rx1 = hub.subscribe();
        let mut rx2 = hub.subscribe();

        let task = create_test_task();
        let task_id = task.id;
        hub.broadcast(SseEvent::task_created(task));

        let received1 = rx1.recv().await.expect("Subscriber 1 should receive");
        let received2 = rx2.recv().await.expect("Subscriber 2 should receive");

        if let SseEvent::TaskCreated { task: t1 } = received1 {
            assert_eq!(t1.id, task_id);
        }
        if let SseEvent::TaskCreated { task: t2 } = received2 {
            assert_eq!(t2.id, task_id);
        }
    }

    #[test]
    fn test_hub_default() {
        let hub = SseHub::default();
        // Default capacity is 256, but we can't directly check it
        // Just ensure it creates without panic
        let _rx = hub.subscribe();
    }
}
