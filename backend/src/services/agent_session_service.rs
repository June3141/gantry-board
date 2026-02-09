use chrono::{DateTime, Utc};
use sqlx::prelude::FromRow;
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::error::{AppError, AppResult};
use crate::models::agent_session::{
    AgentSession, AgentSessionStatus, AgentType, CreateAgentSessionRequest,
    UpdateAgentSessionRequest,
};

#[derive(FromRow)]
struct AgentSessionRow {
    id: String,
    task_id: String,
    agent_type: AgentType,
    status: AgentSessionStatus,
    started_at: Option<DateTime<Utc>>,
    finished_at: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl TryFrom<AgentSessionRow> for AgentSession {
    type Error = uuid::Error;

    fn try_from(row: AgentSessionRow) -> Result<Self, Self::Error> {
        Ok(AgentSession {
            id: row.id.parse()?,
            task_id: row.task_id.parse()?,
            agent_type: row.agent_type,
            status: row.status,
            started_at: row.started_at,
            finished_at: row.finished_at,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
    }
}

pub async fn create_agent_session(
    _pool: &SqlitePool,
    _task_id: Uuid,
    _req: &CreateAgentSessionRequest,
) -> AppResult<AgentSession> {
    todo!()
}

pub async fn get_agent_session(
    _pool: &SqlitePool,
    _id: Uuid,
) -> AppResult<AgentSession> {
    todo!()
}

pub async fn list_agent_sessions(
    _pool: &SqlitePool,
    _task_id: Uuid,
) -> AppResult<Vec<AgentSession>> {
    todo!()
}

pub async fn update_agent_session(
    _pool: &SqlitePool,
    _id: Uuid,
    _req: &UpdateAgentSessionRequest,
) -> AppResult<AgentSession> {
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::project::CreateProjectRequest;
    use crate::models::task::CreateTaskRequest;
    use crate::services::{project_service, task_service};
    use crate::test_helpers::setup_test_db;

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

        let session = get_agent_session(&pool, created.id)
            .await
            .expect("Failed to get");

        assert_eq!(session.id, created.id);
        assert_eq!(session.task_id, task_id);
    }

    #[tokio::test]
    async fn test_get_agent_session_returns_not_found() {
        let pool = setup_test_db().await;
        let random_id = Uuid::new_v4();

        let result = get_agent_session(&pool, random_id).await;

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
    async fn test_list_agent_sessions_returns_sessions_for_task() {
        let pool = setup_test_db().await;
        let (_project_id, task_id) = create_test_task(&pool).await;

        let req = CreateAgentSessionRequest {
            agent_type: AgentType::ClaudeCode,
        };
        create_agent_session(&pool, task_id, &req)
            .await
            .expect("Failed to create 1");
        create_agent_session(&pool, task_id, &req)
            .await
            .expect("Failed to create 2");

        let sessions = list_agent_sessions(&pool, task_id)
            .await
            .expect("Failed to list");

        assert_eq!(sessions.len(), 2);
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

        // First transition to running
        update_agent_session(
            &pool,
            created.id,
            &UpdateAgentSessionRequest {
                status: AgentSessionStatus::Running,
            },
        )
        .await
        .expect("Failed to update to running");

        // Then complete
        let updated = update_agent_session(
            &pool,
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
    async fn test_update_status_to_failed_sets_finished_at() {
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
            created.id,
            &UpdateAgentSessionRequest {
                status: AgentSessionStatus::Failed,
            },
        )
        .await
        .expect("Failed to update");

        assert_eq!(updated.status, AgentSessionStatus::Failed);
        assert!(updated.finished_at.is_some());
    }

    #[tokio::test]
    async fn test_update_nonexistent_session_returns_not_found() {
        let pool = setup_test_db().await;
        let random_id = Uuid::new_v4();

        let result = update_agent_session(
            &pool,
            random_id,
            &UpdateAgentSessionRequest {
                status: AgentSessionStatus::Running,
            },
        )
        .await;

        assert!(matches!(result, Err(AppError::NotFound(_))));
    }
}
