use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::models::task::Task;

/// Server-Sent Event for real-time updates
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(tag = "type")]
pub enum SseEvent {
    TaskCreated { task: Task },
    TaskUpdated { task: Task },
    TaskDeleted { task_id: Uuid },
}

impl SseEvent {
    pub fn task_created(task: Task) -> Self {
        Self::TaskCreated { task }
    }

    pub fn task_updated(task: Task) -> Self {
        Self::TaskUpdated { task }
    }

    pub fn task_deleted(task_id: Uuid) -> Self {
        Self::TaskDeleted { task_id }
    }

    pub fn event_type(&self) -> &'static str {
        match self {
            Self::TaskCreated { .. } => "task_created",
            Self::TaskUpdated { .. } => "task_updated",
            Self::TaskDeleted { .. } => "task_deleted",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::task::{TaskPriority, TaskStatus};
    use chrono::Utc;

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
    fn test_task_created_event() {
        let task = create_test_task();
        let event = SseEvent::task_created(task.clone());

        assert_eq!(event.event_type(), "task_created");
        if let SseEvent::TaskCreated { task: t } = event {
            assert_eq!(t.id, task.id);
        } else {
            panic!("Expected TaskCreated event");
        }
    }

    #[test]
    fn test_task_updated_event() {
        let task = create_test_task();
        let event = SseEvent::task_updated(task.clone());

        assert_eq!(event.event_type(), "task_updated");
    }

    #[test]
    fn test_task_deleted_event() {
        let task_id = Uuid::new_v4();
        let event = SseEvent::task_deleted(task_id);

        assert_eq!(event.event_type(), "task_deleted");
        if let SseEvent::TaskDeleted { task_id: id } = event {
            assert_eq!(id, task_id);
        } else {
            panic!("Expected TaskDeleted event");
        }
    }

    #[test]
    fn test_event_serialization() {
        let task = create_test_task();
        let event = SseEvent::task_created(task);

        let json = serde_json::to_string(&event).expect("Failed to serialize");
        assert!(json.contains("\"type\":\"TaskCreated\""));
        assert!(json.contains("\"task\""));
    }
}
