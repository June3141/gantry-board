use axum::http::StatusCode;
use axum_test::TestServer;
use serde_json::json;
use uuid::Uuid;

use crate::common::{create_test_server_with_repo_and_pool, SqlitePool};
use gantry_board::services::agent_session_output_service;

async fn create_test_task(server: &TestServer) -> (String, String) {
    let response = server
        .post("/api/projects")
        .json(&json!({ "name": "Test Project" }))
        .await;
    let project_id = response.json::<serde_json::Value>()["id"]
        .as_str()
        .unwrap()
        .to_string();

    let response = server
        .post("/api/tasks")
        .json(&json!({
            "project_id": project_id,
            "title": "Test Task"
        }))
        .await;
    let task_id = response.json::<serde_json::Value>()["id"]
        .as_str()
        .unwrap()
        .to_string();

    (project_id, task_id)
}

async fn create_session_with_outputs(
    server: &TestServer,
    pool: &SqlitePool,
    task_id: &str,
) -> String {
    let response = server
        .post(&format!("/api/tasks/{}/sessions", task_id))
        .json(&json!({ "agent_type": "claude_code" }))
        .await;
    response.assert_status(StatusCode::CREATED);
    let created: serde_json::Value = response.json();
    let session_id = created["id"].as_str().unwrap().to_string();

    let session_uuid: Uuid = session_id.parse().unwrap();
    for i in 0..3 {
        agent_session_output_service::append_output(
            pool,
            session_uuid,
            i,
            &format!("output chunk {}", i),
        )
        .await
        .expect("Failed to insert test output");
    }

    session_id
}

#[tokio::test]
async fn test_get_session_outputs_returns_200() {
    let (_tmp, server, pool) = create_test_server_with_repo_and_pool().await;
    let (_project_id, task_id) = create_test_task(&server).await;
    let session_id = create_session_with_outputs(&server, &pool, &task_id).await;

    let response = server
        .get(&format!(
            "/api/tasks/{}/sessions/{}/outputs",
            task_id, session_id
        ))
        .await;

    response.assert_status_ok();
    let outputs: Vec<serde_json::Value> = response.json();
    assert_eq!(outputs.len(), 3);
    assert_eq!(outputs[0]["sequence"], 0);
    assert_eq!(outputs[0]["content"], "output chunk 0");
    assert_eq!(outputs[1]["sequence"], 1);
    assert_eq!(outputs[2]["sequence"], 2);
}

#[tokio::test]
async fn test_get_session_outputs_with_after_param() {
    let (_tmp, server, pool) = create_test_server_with_repo_and_pool().await;
    let (_project_id, task_id) = create_test_task(&server).await;
    let session_id = create_session_with_outputs(&server, &pool, &task_id).await;

    let response = server
        .get(&format!(
            "/api/tasks/{}/sessions/{}/outputs?after=1",
            task_id, session_id
        ))
        .await;

    response.assert_status_ok();
    let outputs: Vec<serde_json::Value> = response.json();
    assert_eq!(outputs.len(), 1);
    assert_eq!(outputs[0]["sequence"], 2);
    assert_eq!(outputs[0]["content"], "output chunk 2");
}

#[tokio::test]
async fn test_get_session_outputs_empty_session() {
    let (_tmp, server, _pool) = create_test_server_with_repo_and_pool().await;
    let (_project_id, task_id) = create_test_task(&server).await;

    let create_response = server
        .post(&format!("/api/tasks/{}/sessions", task_id))
        .json(&json!({ "agent_type": "claude_code" }))
        .await;
    create_response.assert_status(StatusCode::CREATED);
    let created: serde_json::Value = create_response.json();
    let session_id = created["id"].as_str().unwrap();

    let response = server
        .get(&format!(
            "/api/tasks/{}/sessions/{}/outputs",
            task_id, session_id
        ))
        .await;

    response.assert_status_ok();
    let outputs: Vec<serde_json::Value> = response.json();
    assert!(outputs.is_empty());
}

#[tokio::test]
async fn test_get_session_outputs_404_for_nonexistent_session() {
    let (_tmp, server, _pool) = create_test_server_with_repo_and_pool().await;
    let (_project_id, task_id) = create_test_task(&server).await;
    let fake_session_id = Uuid::new_v4();

    let response = server
        .get(&format!(
            "/api/tasks/{}/sessions/{}/outputs",
            task_id, fake_session_id
        ))
        .await;

    response.assert_status(StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_get_session_outputs_404_for_nonexistent_task() {
    let (_tmp, server, _pool) = create_test_server_with_repo_and_pool().await;
    let fake_task_id = Uuid::new_v4();
    let fake_session_id = Uuid::new_v4();

    let response = server
        .get(&format!(
            "/api/tasks/{}/sessions/{}/outputs",
            fake_task_id, fake_session_id
        ))
        .await;

    response.assert_status(StatusCode::NOT_FOUND);
}
