use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::models::agent_session::AgentSession;
use crate::models::docker_preview::DockerPreview;
use crate::models::github::SyncResult;
use crate::models::project_message::ProjectMessage;
use crate::models::task::Task;
use crate::models::task_comment::TaskComment;

/// Server-Sent Event for real-time updates
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(tag = "type")]
pub enum SseEvent {
    TaskCreated { task: Task },
    TaskUpdated { task: Task },
    TaskDeleted { task_id: Uuid },
    AgentOutput { session_id: Uuid, text: String },
    AgentSessionStatusChanged { session: AgentSession },
    CommentCreated { comment: TaskComment },
    CommentUpdated { comment: TaskComment },
    CommentDeleted { comment_id: Uuid, task_id: Uuid },
    ProjectMessageCreated { message: ProjectMessage },
    ProjectMessageDeleted { message_id: Uuid, project_id: Uuid },
    PreviewStatusChanged { preview: DockerPreview },
    PreviewDeleted { preview_id: Uuid },
    GitHubSyncCompleted { result: SyncResult },
    GitHubSyncFailed { project_id: Uuid, error: String },
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

    pub fn agent_output(session_id: Uuid, text: String) -> Self {
        Self::AgentOutput { session_id, text }
    }

    pub fn agent_session_status_changed(session: AgentSession) -> Self {
        Self::AgentSessionStatusChanged { session }
    }

    pub fn comment_created(comment: TaskComment) -> Self {
        Self::CommentCreated { comment }
    }

    pub fn comment_updated(comment: TaskComment) -> Self {
        Self::CommentUpdated { comment }
    }

    pub fn comment_deleted(comment_id: Uuid, task_id: Uuid) -> Self {
        Self::CommentDeleted {
            comment_id,
            task_id,
        }
    }

    pub fn project_message_created(message: ProjectMessage) -> Self {
        Self::ProjectMessageCreated { message }
    }

    pub fn project_message_deleted(message_id: Uuid, project_id: Uuid) -> Self {
        Self::ProjectMessageDeleted {
            message_id,
            project_id,
        }
    }

    pub fn preview_status_changed(preview: DockerPreview) -> Self {
        Self::PreviewStatusChanged { preview }
    }

    pub fn preview_deleted(preview_id: Uuid) -> Self {
        Self::PreviewDeleted { preview_id }
    }

    pub fn github_sync_completed(result: SyncResult) -> Self {
        Self::GitHubSyncCompleted { result }
    }

    pub fn github_sync_failed(project_id: Uuid, error: String) -> Self {
        Self::GitHubSyncFailed { project_id, error }
    }

    pub fn event_type(&self) -> &'static str {
        match self {
            Self::TaskCreated { .. } => "task_created",
            Self::TaskUpdated { .. } => "task_updated",
            Self::TaskDeleted { .. } => "task_deleted",
            Self::AgentOutput { .. } => "agent_output",
            Self::AgentSessionStatusChanged { .. } => "agent_session_status_changed",
            Self::CommentCreated { .. } => "comment_created",
            Self::CommentUpdated { .. } => "comment_updated",
            Self::CommentDeleted { .. } => "comment_deleted",
            Self::ProjectMessageCreated { .. } => "project_message_created",
            Self::ProjectMessageDeleted { .. } => "project_message_deleted",
            Self::PreviewStatusChanged { .. } => "preview_status_changed",
            Self::PreviewDeleted { .. } => "preview_deleted",
            Self::GitHubSyncCompleted { .. } => "github_sync_completed",
            Self::GitHubSyncFailed { .. } => "github_sync_failed",
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

    #[test]
    fn test_agent_output_event() {
        let session_id = Uuid::new_v4();
        let event = SseEvent::agent_output(session_id, "Hello from agent".to_string());

        assert_eq!(event.event_type(), "agent_output");
        if let SseEvent::AgentOutput {
            session_id: sid,
            text,
        } = event
        {
            assert_eq!(sid, session_id);
            assert_eq!(text, "Hello from agent");
        } else {
            panic!("Expected AgentOutput event");
        }
    }

    #[test]
    fn test_agent_output_serialization() {
        let session_id = Uuid::new_v4();
        let event = SseEvent::agent_output(session_id, "test output".to_string());

        let json = serde_json::to_string(&event).expect("Failed to serialize");
        assert!(json.contains("\"type\":\"AgentOutput\""));
        assert!(json.contains("\"text\":\"test output\""));
        assert!(json.contains(&session_id.to_string()));
    }

    #[test]
    fn test_github_sync_completed_event() {
        let result = crate::models::github::SyncResult {
            project_id: Uuid::new_v4(),
            pushed: 3,
            pulled: 2,
        };
        let event = SseEvent::github_sync_completed(result.clone());
        assert_eq!(event.event_type(), "github_sync_completed");
        if let SseEvent::GitHubSyncCompleted { result: r } = event {
            assert_eq!(r.pushed, 3);
            assert_eq!(r.pulled, 2);
        } else {
            panic!("Expected GitHubSyncCompleted event");
        }
    }

    #[test]
    fn test_github_sync_failed_event() {
        let project_id = Uuid::new_v4();
        let event = SseEvent::github_sync_failed(project_id, "API error".to_string());
        assert_eq!(event.event_type(), "github_sync_failed");
        if let SseEvent::GitHubSyncFailed {
            project_id: pid,
            error,
        } = event
        {
            assert_eq!(pid, project_id);
            assert_eq!(error, "API error");
        } else {
            panic!("Expected GitHubSyncFailed event");
        }
    }

    #[test]
    fn test_agent_session_status_changed_event() {
        use crate::models::agent_session::{AgentSessionStatus, AgentType};

        let session = AgentSession {
            id: Uuid::new_v4(),
            task_id: Uuid::new_v4(),
            agent_type: AgentType::ClaudeCode,
            status: AgentSessionStatus::Running,
            prompt: None,
            worktree_name: None,
            started_at: Some(Utc::now()),
            finished_at: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        let event = SseEvent::agent_session_status_changed(session.clone());

        assert_eq!(event.event_type(), "agent_session_status_changed");
        if let SseEvent::AgentSessionStatusChanged { session: s } = event {
            assert_eq!(s.id, session.id);
            assert_eq!(s.status, AgentSessionStatus::Running);
        } else {
            panic!("Expected AgentSessionStatusChanged event");
        }
    }
}
