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
    pool: &SqlitePool,
    task_id: Uuid,
    req: &CreateAgentSessionRequest,
) -> AppResult<AgentSession> {
    let id = Uuid::new_v4();
    let now = Utc::now();

    sqlx::query(
        r#"
        INSERT INTO agent_sessions (id, task_id, agent_type, status, created_at, updated_at)
        VALUES ($1, $2, $3, $4, $5, $6)
        "#,
    )
    .bind(id.to_string())
    .bind(task_id.to_string())
    .bind(&req.agent_type)
    .bind(&AgentSessionStatus::Pending)
    .bind(now)
    .bind(now)
    .execute(pool)
    .await?;

    Ok(AgentSession {
        id,
        task_id,
        agent_type: req.agent_type.clone(),
        status: AgentSessionStatus::Pending,
        started_at: None,
        finished_at: None,
        created_at: now,
        updated_at: now,
    })
}

pub async fn get_agent_session(
    pool: &SqlitePool,
    task_id: Uuid,
    id: Uuid,
) -> AppResult<AgentSession> {
    let row = sqlx::query_as::<_, AgentSessionRow>(
        r#"
        SELECT id, task_id, agent_type, status, started_at, finished_at, created_at, updated_at
        FROM agent_sessions
        WHERE id = $1 AND task_id = $2
        "#,
    )
    .bind(id.to_string())
    .bind(task_id.to_string())
    .fetch_optional(pool)
    .await?;

    row.map(|r| r.try_into())
        .transpose()
        .map_err(|e: uuid::Error| AppError::Internal(e.to_string()))?
        .ok_or_else(|| AppError::NotFound(format!("agent session {} not found", id)))
}

pub async fn list_agent_sessions(pool: &SqlitePool, task_id: Uuid) -> AppResult<Vec<AgentSession>> {
    let rows = sqlx::query_as::<_, AgentSessionRow>(
        r#"
        SELECT id, task_id, agent_type, status, started_at, finished_at, created_at, updated_at
        FROM agent_sessions
        WHERE task_id = $1
        ORDER BY created_at ASC
        "#,
    )
    .bind(task_id.to_string())
    .fetch_all(pool)
    .await?;

    rows.into_iter()
        .map(|r| r.try_into())
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e: uuid::Error| AppError::Internal(e.to_string()))
}

fn validate_status_transition(from: &AgentSessionStatus, to: &AgentSessionStatus) -> AppResult<()> {
    use AgentSessionStatus::*;
    let allowed = matches!(
        (from, to),
        (Pending, Running)
            | (Pending, Cancelled)
            | (Running, Completed)
            | (Running, Failed)
            | (Running, Cancelled)
    );
    if !allowed {
        return Err(AppError::Validation(format!(
            "invalid status transition from {} to {}",
            serde_json::to_value(from)
                .unwrap_or_default()
                .as_str()
                .unwrap_or("unknown"),
            serde_json::to_value(to)
                .unwrap_or_default()
                .as_str()
                .unwrap_or("unknown"),
        )));
    }
    Ok(())
}

pub async fn update_agent_session(
    pool: &SqlitePool,
    task_id: Uuid,
    id: Uuid,
    req: &UpdateAgentSessionRequest,
) -> AppResult<AgentSession> {
    let existing = get_agent_session(pool, task_id, id).await?;
    validate_status_transition(&existing.status, &req.status)?;
    let now = Utc::now();

    let started_at = match req.status {
        AgentSessionStatus::Running => existing.started_at.or(Some(now)),
        _ => existing.started_at,
    };

    let finished_at = match req.status {
        AgentSessionStatus::Completed
        | AgentSessionStatus::Failed
        | AgentSessionStatus::Cancelled => Some(existing.finished_at.unwrap_or(now)),
        _ => None,
    };

    sqlx::query(
        r#"
        UPDATE agent_sessions
        SET status = $1, started_at = $2, finished_at = $3, updated_at = $4
        WHERE id = $5 AND task_id = $6
        "#,
    )
    .bind(&req.status)
    .bind(started_at)
    .bind(finished_at)
    .bind(now)
    .bind(id.to_string())
    .bind(task_id.to_string())
    .execute(pool)
    .await?;

    Ok(AgentSession {
        id,
        task_id: existing.task_id,
        agent_type: existing.agent_type,
        status: req.status.clone(),
        started_at,
        finished_at,
        created_at: existing.created_at,
        updated_at: now,
    })
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
    async fn test_get_agent_session_wrong_task_returns_not_found() {
        let pool = setup_test_db().await;
        let (_project_id, task_id) = create_test_task(&pool).await;
        let (_project_id2, other_task_id) = create_test_task(&pool).await;

        let created = create_agent_session(
            &pool,
            task_id,
            &CreateAgentSessionRequest {
                agent_type: AgentType::ClaudeCode,
            },
        )
        .await
        .expect("Failed to create");

        let result = get_agent_session(&pool, other_task_id, created.id).await;

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
        // Create first session and complete it so we can create another
        let session1 = create_agent_session(&pool, task_id, &req)
            .await
            .expect("Failed to create 1");
        update_agent_session(
            &pool,
            task_id,
            session1.id,
            &UpdateAgentSessionRequest {
                status: AgentSessionStatus::Running,
            },
        )
        .await
        .expect("Failed to update to running");
        update_agent_session(
            &pool,
            task_id,
            session1.id,
            &UpdateAgentSessionRequest {
                status: AgentSessionStatus::Completed,
            },
        )
        .await
        .expect("Failed to update to completed");

        // Now create second session (allowed because first is completed)
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

        // First transition to running
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

        // Then complete
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

        // Pending -> Running first
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
        let (_project_id, task_id) = create_test_task(&pool).await;
        let random_id = Uuid::new_v4();

        let result = update_agent_session(
            &pool,
            task_id,
            random_id,
            &UpdateAgentSessionRequest {
                status: AgentSessionStatus::Running,
            },
        )
        .await;

        assert!(matches!(result, Err(AppError::NotFound(_))));
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

        // Pending -> Running -> Completed
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

        // Completed -> Running should fail
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
    async fn test_concurrent_pending_sessions_blocked_by_unique_constraint() {
        let pool = setup_test_db().await;
        let (_project_id, task_id) = create_test_task(&pool).await;

        let req = CreateAgentSessionRequest {
            agent_type: AgentType::ClaudeCode,
        };

        // First session: OK
        create_agent_session(&pool, task_id, &req)
            .await
            .expect("First session should succeed");

        // Second session on same task while first is still pending: should fail
        let result = create_agent_session(&pool, task_id, &req).await;
        assert!(result.is_err(), "Second pending session should be rejected");
    }

    #[tokio::test]
    async fn test_new_session_allowed_after_previous_completed() {
        let pool = setup_test_db().await;
        let (_project_id, task_id) = create_test_task(&pool).await;

        let req = CreateAgentSessionRequest {
            agent_type: AgentType::ClaudeCode,
        };

        let session = create_agent_session(&pool, task_id, &req)
            .await
            .expect("First session should succeed");

        // Complete the first session
        update_agent_session(
            &pool,
            task_id,
            session.id,
            &UpdateAgentSessionRequest {
                status: AgentSessionStatus::Running,
            },
        )
        .await
        .expect("Transition to running");
        update_agent_session(
            &pool,
            task_id,
            session.id,
            &UpdateAgentSessionRequest {
                status: AgentSessionStatus::Completed,
            },
        )
        .await
        .expect("Transition to completed");

        // New session should now succeed
        let result = create_agent_session(&pool, task_id, &req).await;
        assert!(
            result.is_ok(),
            "New session after completion should succeed"
        );
    }

    #[tokio::test]
    async fn test_new_session_allowed_after_previous_cancelled() {
        let pool = setup_test_db().await;
        let (_project_id, task_id) = create_test_task(&pool).await;

        let req = CreateAgentSessionRequest {
            agent_type: AgentType::ClaudeCode,
        };

        let session = create_agent_session(&pool, task_id, &req)
            .await
            .expect("First session should succeed");

        // Cancel the first session
        update_agent_session(
            &pool,
            task_id,
            session.id,
            &UpdateAgentSessionRequest {
                status: AgentSessionStatus::Cancelled,
            },
        )
        .await
        .expect("Transition to cancelled");

        // New session should now succeed
        let result = create_agent_session(&pool, task_id, &req).await;
        assert!(
            result.is_ok(),
            "New session after cancellation should succeed"
        );
    }

    #[tokio::test]
    async fn test_running_session_blocks_new_session_creation() {
        let pool = setup_test_db().await;
        let (_project_id, task_id) = create_test_task(&pool).await;

        let req = CreateAgentSessionRequest {
            agent_type: AgentType::ClaudeCode,
        };

        let session = create_agent_session(&pool, task_id, &req)
            .await
            .expect("First session should succeed");

        // Transition to running
        update_agent_session(
            &pool,
            task_id,
            session.id,
            &UpdateAgentSessionRequest {
                status: AgentSessionStatus::Running,
            },
        )
        .await
        .expect("Transition to running");

        // Second session while first is running: should fail
        let result = create_agent_session(&pool, task_id, &req).await;
        assert!(result.is_err(), "Session during running should be rejected");
    }

    #[tokio::test]
    async fn test_new_session_allowed_after_previous_failed() {
        let pool = setup_test_db().await;
        let (_project_id, task_id) = create_test_task(&pool).await;

        let req = CreateAgentSessionRequest {
            agent_type: AgentType::ClaudeCode,
        };

        let session = create_agent_session(&pool, task_id, &req)
            .await
            .expect("First session should succeed");

        // Transition: pending -> running -> failed
        update_agent_session(
            &pool,
            task_id,
            session.id,
            &UpdateAgentSessionRequest {
                status: AgentSessionStatus::Running,
            },
        )
        .await
        .expect("Transition to running");
        update_agent_session(
            &pool,
            task_id,
            session.id,
            &UpdateAgentSessionRequest {
                status: AgentSessionStatus::Failed,
            },
        )
        .await
        .expect("Transition to failed");

        // New session should now succeed
        let result = create_agent_session(&pool, task_id, &req).await;
        assert!(result.is_ok(), "New session after failure should succeed");
    }

    #[tokio::test]
    async fn test_invalid_transition_pending_to_completed_returns_validation_error() {
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

        // Pending -> Completed should fail (must go through Running)
        let result = update_agent_session(
            &pool,
            task_id,
            created.id,
            &UpdateAgentSessionRequest {
                status: AgentSessionStatus::Completed,
            },
        )
        .await;

        assert!(matches!(result, Err(AppError::Validation(_))));
    }
}
