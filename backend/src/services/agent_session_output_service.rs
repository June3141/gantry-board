use sqlx::SqlitePool;
use uuid::Uuid;

use crate::error::{AppError, AppResult};
use crate::models::agent_session_output::{AgentSessionOutput, AgentSessionOutputRow};

pub async fn append_output(
    _pool: &SqlitePool,
    _session_id: Uuid,
    _sequence: i64,
    _content: &str,
) -> AppResult<AgentSessionOutput> {
    todo!()
}

pub async fn get_outputs(
    _pool: &SqlitePool,
    _session_id: Uuid,
) -> AppResult<Vec<AgentSessionOutput>> {
    todo!()
}

pub async fn get_outputs_after(
    _pool: &SqlitePool,
    _session_id: Uuid,
    _after_sequence: i64,
) -> AppResult<Vec<AgentSessionOutput>> {
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::agent_session::{AgentType, CreateAgentSessionRequest};
    use crate::models::project::CreateProjectRequest;
    use crate::models::task::CreateTaskRequest;
    use crate::services::{agent_session_service, project_service, task_service};
    use crate::test_helpers::setup_test_db;

    async fn create_test_session(pool: &SqlitePool) -> Uuid {
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

        let session = agent_session_service::create_agent_session(
            pool,
            task.id,
            &CreateAgentSessionRequest {
                agent_type: AgentType::ClaudeCode,
            },
        )
        .await
        .expect("Failed to create session");

        session.id
    }

    #[tokio::test]
    async fn test_append_and_get_outputs_roundtrip() {
        let pool = setup_test_db().await;
        let session_id = create_test_session(&pool).await;

        let out1 = append_output(&pool, session_id, 0, "Hello ")
            .await
            .expect("Failed to append");
        let out2 = append_output(&pool, session_id, 1, "World")
            .await
            .expect("Failed to append");

        assert_eq!(out1.sequence, 0);
        assert_eq!(out1.content, "Hello ");
        assert_eq!(out1.session_id, session_id);
        assert_eq!(out2.sequence, 1);
        assert_eq!(out2.content, "World");

        let outputs = get_outputs(&pool, session_id).await.expect("Failed to get");
        assert_eq!(outputs.len(), 2);
        assert_eq!(outputs[0].sequence, 0);
        assert_eq!(outputs[0].content, "Hello ");
        assert_eq!(outputs[1].sequence, 1);
        assert_eq!(outputs[1].content, "World");
    }

    #[tokio::test]
    async fn test_get_outputs_returns_ordered_by_sequence() {
        let pool = setup_test_db().await;
        let session_id = create_test_session(&pool).await;

        append_output(&pool, session_id, 2, "Third")
            .await
            .expect("Failed");
        append_output(&pool, session_id, 0, "First")
            .await
            .expect("Failed");
        append_output(&pool, session_id, 1, "Second")
            .await
            .expect("Failed");

        let outputs = get_outputs(&pool, session_id).await.expect("Failed to get");
        assert_eq!(outputs.len(), 3);
        assert_eq!(outputs[0].content, "First");
        assert_eq!(outputs[1].content, "Second");
        assert_eq!(outputs[2].content, "Third");
    }

    #[tokio::test]
    async fn test_get_outputs_empty_session() {
        let pool = setup_test_db().await;
        let session_id = create_test_session(&pool).await;

        let outputs = get_outputs(&pool, session_id).await.expect("Failed to get");
        assert!(outputs.is_empty());
    }

    #[tokio::test]
    async fn test_get_outputs_after_sequence() {
        let pool = setup_test_db().await;
        let session_id = create_test_session(&pool).await;

        for i in 0..5 {
            append_output(&pool, session_id, i, &format!("chunk-{}", i))
                .await
                .expect("Failed");
        }

        let outputs = get_outputs_after(&pool, session_id, 2)
            .await
            .expect("Failed to get");
        assert_eq!(outputs.len(), 2);
        assert_eq!(outputs[0].sequence, 3);
        assert_eq!(outputs[1].sequence, 4);
    }

    #[tokio::test]
    async fn test_duplicate_sequence_returns_error() {
        let pool = setup_test_db().await;
        let session_id = create_test_session(&pool).await;

        append_output(&pool, session_id, 0, "first")
            .await
            .expect("First insert should succeed");

        let result = append_output(&pool, session_id, 0, "duplicate").await;
        assert!(result.is_err(), "Duplicate sequence should fail");
    }
}
