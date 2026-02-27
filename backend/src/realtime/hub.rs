use tokio::sync::broadcast;
use uuid::Uuid;

use super::event::SseEvent;

/// Receiver that filters events by project_id.
///
/// Events whose `project_id()` matches the filter are forwarded.
/// Events without a project_id (returning `None`) are also forwarded,
/// since they cannot be attributed to a specific project.
pub struct ProjectReceiver {
    inner: broadcast::Receiver<SseEvent>,
    project_id: Uuid,
}

impl ProjectReceiver {
    /// Receive the next event that matches this project filter.
    ///
    /// Skips events destined for other projects. Returns `Err` on
    /// channel close or lag (same semantics as `broadcast::Receiver`).
    pub async fn recv(&mut self) -> Result<SseEvent, broadcast::error::RecvError> {
        loop {
            let event = self.inner.recv().await?;
            match event.project_id() {
                Some(pid) if pid != self.project_id => continue,
                _ => return Ok(event),
            }
        }
    }
}

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

    /// Subscribe to events for a specific project.
    ///
    /// The returned receiver filters out events destined for other projects.
    /// Events without a project_id are still forwarded.
    pub fn subscribe_project(&self, project_id: Uuid) -> ProjectReceiver {
        ProjectReceiver {
            inner: self.sender.subscribe(),
            project_id,
        }
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

    #[tokio::test]
    async fn test_project_subscriber_receives_matching_events_only() {
        let hub = SseHub::new(16);

        let project_a = Uuid::new_v4();
        let project_b = Uuid::new_v4();

        let mut rx_a = hub.subscribe_project(project_a);
        let mut rx_b = hub.subscribe_project(project_b);

        // Create tasks for project A and project B
        let task_a = Task {
            id: Uuid::new_v4(),
            project_id: project_a,
            title: "Task A".to_string(),
            description: None,
            status: TaskStatus::Todo,
            priority: TaskPriority::Medium,
            parent_id: None,
            assigned_to: None,
            position: 0,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        let task_b = Task {
            id: Uuid::new_v4(),
            project_id: project_b,
            title: "Task B".to_string(),
            description: None,
            status: TaskStatus::Todo,
            priority: TaskPriority::Medium,
            parent_id: None,
            assigned_to: None,
            position: 0,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        hub.broadcast(SseEvent::task_created(task_a.clone()));
        hub.broadcast(SseEvent::task_created(task_b.clone()));

        // Subscriber A should only get task A
        let event = rx_a.recv().await.expect("Should receive project A event");
        if let SseEvent::TaskCreated { task } = event {
            assert_eq!(task.project_id, project_a);
        } else {
            panic!("Expected TaskCreated");
        }

        // Subscriber B should only get task B
        let event = rx_b.recv().await.expect("Should receive project B event");
        if let SseEvent::TaskCreated { task } = event {
            assert_eq!(task.project_id, project_b);
        } else {
            panic!("Expected TaskCreated");
        }
    }

    #[tokio::test]
    async fn test_global_subscriber_receives_all_events() {
        let hub = SseHub::new(16);

        let project_a = Uuid::new_v4();
        let project_b = Uuid::new_v4();

        // Global subscriber (no project filter) should receive everything
        let mut rx_all = hub.subscribe();

        let task_a = Task {
            id: Uuid::new_v4(),
            project_id: project_a,
            title: "Task A".to_string(),
            description: None,
            status: TaskStatus::Todo,
            priority: TaskPriority::Medium,
            parent_id: None,
            assigned_to: None,
            position: 0,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        let task_b = Task {
            id: Uuid::new_v4(),
            project_id: project_b,
            title: "Task B".to_string(),
            description: None,
            status: TaskStatus::Todo,
            priority: TaskPriority::Medium,
            parent_id: None,
            assigned_to: None,
            position: 0,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        hub.broadcast(SseEvent::task_created(task_a));
        hub.broadcast(SseEvent::task_created(task_b));

        let _ = rx_all.recv().await.expect("Should receive first event");
        let _ = rx_all.recv().await.expect("Should receive second event");
    }

    #[tokio::test]
    async fn test_project_subscriber_receives_events_without_project_id() {
        let hub = SseHub::new(16);
        let project_a = Uuid::new_v4();

        let mut rx_a = hub.subscribe_project(project_a);

        // TaskDeleted has no project_id — project subscribers should still receive it
        // since we cannot determine which project it belongs to
        let task_id = Uuid::new_v4();
        hub.broadcast(SseEvent::task_deleted(task_id));

        let event = rx_a
            .recv()
            .await
            .expect("Should receive event without project_id");
        assert_eq!(event.event_type(), "task_deleted");
    }
}
