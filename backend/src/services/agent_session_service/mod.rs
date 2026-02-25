//! Agent session service — split into query (read) and command (write) submodules.

pub mod command;
pub mod query;

// Re-export all public items for backward compatibility.
pub use command::*;
pub use query::*;

use chrono::{DateTime, Utc};
use sqlx::prelude::FromRow;

use crate::models::agent_session::{AgentSession, AgentSessionStatus, AgentType};

/// Internal row type shared between query and command submodules.
#[derive(FromRow)]
pub(crate) struct AgentSessionRow {
    pub id: String,
    pub task_id: String,
    pub agent_type: AgentType,
    pub status: AgentSessionStatus,
    pub prompt: Option<String>,
    pub worktree_name: Option<String>,
    pub started_at: Option<DateTime<Utc>>,
    pub finished_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl TryFrom<AgentSessionRow> for AgentSession {
    type Error = uuid::Error;

    fn try_from(row: AgentSessionRow) -> Result<Self, Self::Error> {
        Ok(AgentSession {
            id: row.id.parse()?,
            task_id: row.task_id.parse()?,
            agent_type: row.agent_type,
            status: row.status,
            prompt: row.prompt,
            worktree_name: row.worktree_name,
            started_at: row.started_at,
            finished_at: row.finished_at,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::AppError;
    use crate::models::agent_session::{
        AgentSessionStatus, AgentType, CreateAgentSessionRequest, UpdateAgentSessionRequest,
    };
    use crate::models::project::CreateProjectRequest;
    use crate::models::task::CreateTaskRequest;
    use crate::services::{project_service, task_service};
    use crate::test_helpers::setup_test_db;
    use sqlx::SqlitePool;
    use uuid::Uuid;

    async fn create_test_task(pool: &SqlitePool) -> (Uuid, Uuid) {
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

        (project.id, task.id)
    }

    #[tokio::test]
    async fn test_create_agent_session_returns_pending_session() {
        let pool = setup_test_db().await;
        let (_project_id, task_id) = create_test_task(&pool).await;

        let req = CreateAgentSessionRequest {
            agent_type: AgentType::ClaudeCode,
        };
        let session = create_agent_session(&pool, task_id, &req)
            .await
            .expect("Failed to create agent session");

        assert_eq!(session.task_id, task_id);
        assert!(matches!(session.agent_type, AgentType::ClaudeCode));
        assert_eq!(session.status, AgentSessionStatus::Pending);
        assert!(session.started_at.is_none());
        assert!(session.finished_at.is_none());
    }

    #[tokio::test]
    async fn test_create_agent_session_with_gemini_cli() {
        let pool = setup_test_db().await;
        let (_project_id, task_id) = create_test_task(&pool).await;

        let req = CreateAgentSessionRequest {
            agent_type: AgentType::GeminiCli,
        };
        let session = create_agent_session(&pool, task_id, &req)
            .await
            .expect("Failed to create agent session");

        assert!(matches!(session.agent_type, AgentType::GeminiCli));
    }

    #[tokio::test]
    async fn test_create_agent_session_fails_for_nonexistent_task() {
        let pool = setup_test_db().await;
        let fake_task_id = Uuid::new_v4();

        let req = CreateAgentSessionRequest {
            agent_type: AgentType::ClaudeCode,
        };
        let result = create_agent_session(&pool, fake_task_id, &req).await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_get_agent_session_returns_existing() {
        let pool = setup_test_db().await;
        let (_project_id, task_id) = create_test_task(&pool).await;

        let req = CreateAgentSessionRequest {
            agent_type: AgentType::ClaudeCode,
        };
        let created = create_agent_session(&pool, task_id, &req)
            .await
            .expect("Failed to create");

        let session = get_agent_session(&pool, task_id, created.id)
            .await
            .expect("Failed to get");

        assert_eq!(session.id, created.id);
        assert_eq!(session.task_id, task_id);
    }

    #[tokio::test]
    async fn test_get_agent_session_returns_not_found() {
        let pool = setup_test_db().await;
        let (_project_id, task_id) = create_test_task(&pool).await;
        let random_id = Uuid::new_v4();

        let result = get_agent_session(&pool, task_id, random_id).await;

        assert!(matches!(result, Err(AppError::NotFound(_))));
    }

    #[tokio::test]
    async fn test_list_agent_sessions_returns_empty() {
        let pool = setup_test_db().await;
        let (_project_id, task_id) = create_test_task(&pool).await;

        let sessions = list_agent_sessions(&pool, task_id)
            .await
            .expect("Failed to list");

        assert!(sessions.is_empty());
    }

    #[tokio::test]
    async fn test_update_status_to_running_sets_started_at() {
        let pool = setup_test_db().await;
        let (_project_id, task_id) = create_test_task(&pool).await;

        let created = create_agent_session(
            &pool,
            task_id,
            &CreateAgentSessionRequest {
                agent_type: AgentType::ClaudeCode,
            },
        )
        .await
        .expect("Failed to create");

        let updated = update_agent_session(
            &pool,
            task_id,
            created.id,
            &UpdateAgentSessionRequest {
                status: AgentSessionStatus::Running,
            },
        )
        .await
        .expect("Failed to update");

        assert_eq!(updated.status, AgentSessionStatus::Running);
        assert!(updated.started_at.is_some());
        assert!(updated.finished_at.is_none());
    }

    #[tokio::test]
    async fn test_update_status_to_completed_sets_finished_at() {
        let pool = setup_test_db().await;
        let (_project_id, task_id) = create_test_task(&pool).await;

        let created = create_agent_session(
            &pool,
            task_id,
            &CreateAgentSessionRequest {
                agent_type: AgentType::ClaudeCode,
            },
        )
        .await
        .expect("Failed to create");

        update_agent_session(
            &pool,
            task_id,
            created.id,
            &UpdateAgentSessionRequest {
                status: AgentSessionStatus::Running,
            },
        )
        .await
        .expect("Failed to update to running");

        let updated = update_agent_session(
            &pool,
            task_id,
            created.id,
            &UpdateAgentSessionRequest {
                status: AgentSessionStatus::Completed,
            },
        )
        .await
        .expect("Failed to update to completed");

        assert_eq!(updated.status, AgentSessionStatus::Completed);
        assert!(updated.started_at.is_some());
        assert!(updated.finished_at.is_some());
    }

    #[tokio::test]
    async fn test_invalid_transition_completed_to_running_returns_validation_error() {
        let pool = setup_test_db().await;
        let (_project_id, task_id) = create_test_task(&pool).await;

        let created = create_agent_session(
            &pool,
            task_id,
            &CreateAgentSessionRequest {
                agent_type: AgentType::ClaudeCode,
            },
        )
        .await
        .expect("Failed to create");

        update_agent_session(
            &pool,
            task_id,
            created.id,
            &UpdateAgentSessionRequest {
                status: AgentSessionStatus::Running,
            },
        )
        .await
        .expect("Failed");
        update_agent_session(
            &pool,
            task_id,
            created.id,
            &UpdateAgentSessionRequest {
                status: AgentSessionStatus::Completed,
            },
        )
        .await
        .expect("Failed");

        let result = update_agent_session(
            &pool,
            task_id,
            created.id,
            &UpdateAgentSessionRequest {
                status: AgentSessionStatus::Running,
            },
        )
        .await;

        assert!(matches!(result, Err(AppError::Validation(_))));
    }

    #[tokio::test]
    async fn test_recover_orphaned_sessions_marks_active_as_failed() {
        let pool = setup_test_db().await;
        let (_project_id, task_id1) = create_test_task(&pool).await;
        let (_project_id2, task_id2) = create_test_task(&pool).await;

        let req = CreateAgentSessionRequest {
            agent_type: AgentType::ClaudeCode,
        };

        let s1 = create_agent_session(&pool, task_id1, &req)
            .await
            .expect("create s1");
        update_agent_session(
            &pool,
            task_id1,
            s1.id,
            &UpdateAgentSessionRequest {
                status: AgentSessionStatus::Running,
            },
        )
        .await
        .expect("s1 to running");

        let _s2 = create_agent_session(&pool, task_id2, &req)
            .await
            .expect("create s2");

        let recovered = recover_orphaned_sessions(&pool)
            .await
            .expect("recover should succeed");
        assert_eq!(recovered.len(), 2);

        let r1 = get_agent_session(&pool, task_id1, s1.id)
            .await
            .expect("get s1");
        assert_eq!(r1.status, AgentSessionStatus::Failed);
        assert!(r1.finished_at.is_some());
    }

    #[tokio::test]
    async fn test_recover_orphaned_sessions_idempotent() {
        let pool = setup_test_db().await;
        let (_project_id, task_id) = create_test_task(&pool).await;

        let req = CreateAgentSessionRequest {
            agent_type: AgentType::ClaudeCode,
        };

        let s = create_agent_session(&pool, task_id, &req)
            .await
            .expect("create session");
        update_agent_session(
            &pool,
            task_id,
            s.id,
            &UpdateAgentSessionRequest {
                status: AgentSessionStatus::Running,
            },
        )
        .await
        .expect("to running");

        let recovered1 = recover_orphaned_sessions(&pool)
            .await
            .expect("first recover");
        assert_eq!(recovered1.len(), 1);

        let recovered2 = recover_orphaned_sessions(&pool)
            .await
            .expect("second recover");
        assert_eq!(recovered2.len(), 0, "second call should be no-op");
    }
}
