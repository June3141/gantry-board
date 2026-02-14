use axum::http::StatusCode;
use serde_json::json;
use uuid::Uuid;

use crate::common::create_test_server_with_repo as create_test_server;

async fn create_test_task(server: &axum_test::TestServer) -> (String, String) {
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

#[tokio::test]
async fn test_restart_session_creates_new_session() {
    let (_tmp, server) = create_test_server().await;
    let (_project_id, task_id) = create_test_task(&server).await;

    let start_response = server
        .post(&format!("/api/tasks/{}/sessions/start", task_id))
        .json(&json!({
            "agent_type": "claude_code",
            "prompt": "Implement feature X"
        }))
        .await;
    start_response.assert_status(StatusCode::CREATED);
    let start_body: serde_json::Value = start_response.json();
    let session_id = start_body["session"]["id"].as_str().unwrap();

    server
        .post(&format!(
            "/api/tasks/{}/sessions/{}/stop",
            task_id, session_id
        ))
        .await
        .assert_status_ok();

    let restart_response = server
        .post(&format!(
            "/api/tasks/{}/sessions/{}/restart",
            task_id, session_id
        ))
        .await;

    restart_response.assert_status(StatusCode::CREATED);
    let restart_body: serde_json::Value = restart_response.json();
    let new_session = &restart_body["session"];

    assert_ne!(new_session["id"].as_str().unwrap(), session_id);
    assert_eq!(new_session["prompt"], "Implement feature X");
    assert_eq!(new_session["agent_type"], "claude_code");
    assert_eq!(new_session["status"], "running");
}

#[tokio::test]
async fn test_restart_session_409_when_active_session_exists() {
    let (_tmp, server) = create_test_server().await;
    let (_project_id, task_id) = create_test_task(&server).await;

    let start_response = server
        .post(&format!("/api/tasks/{}/sessions/start", task_id))
        .json(&json!({
            "agent_type": "claude_code",
            "prompt": "Implement feature X"
        }))
        .await;
    start_response.assert_status(StatusCode::CREATED);
    let start_body: serde_json::Value = start_response.json();
    let session_id = start_body["session"]["id"].as_str().unwrap();

    let restart_response = server
        .post(&format!(
            "/api/tasks/{}/sessions/{}/restart",
            task_id, session_id
        ))
        .await;

    restart_response.assert_status(StatusCode::CONFLICT);
}

#[tokio::test]
async fn test_restart_session_404_for_nonexistent_session() {
    let (_tmp, server) = create_test_server().await;
    let (_project_id, task_id) = create_test_task(&server).await;
    let fake_session_id = Uuid::new_v4();

    let response = server
        .post(&format!(
            "/api/tasks/{}/sessions/{}/restart",
            task_id, fake_session_id
        ))
        .await;

    response.assert_status(StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_restart_session_400_when_no_prompt_saved() {
    let (_tmp, server) = create_test_server().await;
    let (_project_id, task_id) = create_test_task(&server).await;

    let create_response = server
        .post(&format!("/api/tasks/{}/sessions", task_id))
        .json(&json!({ "agent_type": "claude_code" }))
        .await;
    create_response.assert_status(StatusCode::CREATED);
    let created: serde_json::Value = create_response.json();
    let session_id = created["id"].as_str().unwrap();

    let response = server
        .post(&format!(
            "/api/tasks/{}/sessions/{}/restart",
            task_id, session_id
        ))
        .await;

    response.assert_status(StatusCode::BAD_REQUEST);
}
