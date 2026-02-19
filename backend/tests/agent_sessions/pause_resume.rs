use axum::http::StatusCode;
use serde_json::json;
use uuid::Uuid;

use crate::common::{create_project_and_task, create_test_server_with_sleep_executor};

/// Helper to start an agent session and return session_id string.
async fn start_session(server: &axum_test::TestServer, task_id: &str) -> String {
    let response = server
        .post(&format!("/api/tasks/{}/sessions/start", task_id))
        .json(&json!({
            "agent_type": "claude_code",
            "prompt": "test prompt"
        }))
        .await;
    response.assert_status(StatusCode::CREATED);
    let body: serde_json::Value = response.json();
    body["session"]["id"].as_str().unwrap().to_string()
}

#[tokio::test]
async fn test_pause_agent_session_returns_200() {
    let (_tmp, server) = create_test_server_with_sleep_executor().await;
    let (_project_id, task_id) = create_project_and_task(&server).await;

    let session_id = start_session(&server, &task_id).await;

    let response = server
        .post(&format!(
            "/api/tasks/{}/sessions/{}/pause",
            task_id, session_id
        ))
        .await;

    response.assert_status_ok();
    let body: serde_json::Value = response.json();
    assert_eq!(body["status"], "paused");
}

#[tokio::test]
async fn test_pause_agent_session_404_for_nonrunning_session() {
    let (_tmp, server) = create_test_server_with_sleep_executor().await;
    let (_project_id, task_id) = create_project_and_task(&server).await;
    let fake_session_id = Uuid::new_v4();

    let response = server
        .post(&format!(
            "/api/tasks/{}/sessions/{}/pause",
            task_id, fake_session_id
        ))
        .await;

    response.assert_status(StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_resume_agent_session_returns_200() {
    let (_tmp, server) = create_test_server_with_sleep_executor().await;
    let (_project_id, task_id) = create_project_and_task(&server).await;

    let session_id = start_session(&server, &task_id).await;

    // Pause first
    server
        .post(&format!(
            "/api/tasks/{}/sessions/{}/pause",
            task_id, session_id
        ))
        .await
        .assert_status_ok();

    // Resume
    let response = server
        .post(&format!(
            "/api/tasks/{}/sessions/{}/resume",
            task_id, session_id
        ))
        .await;

    response.assert_status_ok();
    let body: serde_json::Value = response.json();
    assert_eq!(body["status"], "running");
}

#[tokio::test]
async fn test_resume_agent_session_404_for_nonpaused_session() {
    let (_tmp, server) = create_test_server_with_sleep_executor().await;
    let (_project_id, task_id) = create_project_and_task(&server).await;
    let fake_session_id = Uuid::new_v4();

    let response = server
        .post(&format!(
            "/api/tasks/{}/sessions/{}/resume",
            task_id, fake_session_id
        ))
        .await;

    response.assert_status(StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_stop_paused_session_returns_200() {
    let (_tmp, server) = create_test_server_with_sleep_executor().await;
    let (_project_id, task_id) = create_project_and_task(&server).await;

    let session_id = start_session(&server, &task_id).await;

    // Pause first
    server
        .post(&format!(
            "/api/tasks/{}/sessions/{}/pause",
            task_id, session_id
        ))
        .await
        .assert_status_ok();

    // Stop paused session
    let response = server
        .post(&format!(
            "/api/tasks/{}/sessions/{}/stop",
            task_id, session_id
        ))
        .await;

    response.assert_status_ok();
    let body: serde_json::Value = response.json();
    assert_eq!(body["status"], "cancelled");
}

#[tokio::test]
async fn test_pause_already_paused_session_returns_400() {
    let (_tmp, server) = create_test_server_with_sleep_executor().await;
    let (_project_id, task_id) = create_project_and_task(&server).await;

    let session_id = start_session(&server, &task_id).await;

    // Pause
    server
        .post(&format!(
            "/api/tasks/{}/sessions/{}/pause",
            task_id, session_id
        ))
        .await
        .assert_status_ok();

    // Pause again should fail (Paused → Paused is invalid transition)
    let response = server
        .post(&format!(
            "/api/tasks/{}/sessions/{}/pause",
            task_id, session_id
        ))
        .await;

    response.assert_status(StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_start_new_session_blocked_while_paused() {
    let (_tmp, server) = create_test_server_with_sleep_executor().await;
    let (_project_id, task_id) = create_project_and_task(&server).await;

    let session_id = start_session(&server, &task_id).await;

    // Pause
    server
        .post(&format!(
            "/api/tasks/{}/sessions/{}/pause",
            task_id, session_id
        ))
        .await
        .assert_status_ok();

    // Starting a new session should be blocked
    let response = server
        .post(&format!("/api/tasks/{}/sessions/start", task_id))
        .json(&json!({
            "agent_type": "claude_code",
            "prompt": "another prompt"
        }))
        .await;

    response.assert_status(StatusCode::CONFLICT);
}
