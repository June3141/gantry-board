use axum::http::StatusCode;
use serde_json::json;
use uuid::Uuid;

use crate::common::{create_project_and_task, create_test_server_with_repo as create_test_server};

#[tokio::test]
async fn test_start_agent_session_returns_created() {
    let (_tmp, server) = create_test_server().await;
    let (_project_id, task_id) = create_project_and_task(&server).await;

    let response = server
        .post(&format!("/api/tasks/{}/sessions/start", task_id))
        .json(&json!({
            "agent_type": "claude_code",
            "prompt": "Fix the bug in main.rs"
        }))
        .await;

    response.assert_status(StatusCode::CREATED);
    let body: serde_json::Value = response.json();
    let session = &body["session"];
    assert_eq!(session["task_id"], task_id);
    assert_eq!(session["agent_type"], "claude_code");
    assert_eq!(session["status"], "running");
    assert!(!session["started_at"].is_null());
}

#[tokio::test]
async fn test_start_agent_session_409_when_active_session_exists() {
    let (_tmp, server) = create_test_server().await;
    let (_project_id, task_id) = create_project_and_task(&server).await;

    server
        .post(&format!("/api/tasks/{}/sessions/start", task_id))
        .json(&json!({
            "agent_type": "claude_code",
            "prompt": "First task"
        }))
        .await
        .assert_status(StatusCode::CREATED);

    let response = server
        .post(&format!("/api/tasks/{}/sessions/start", task_id))
        .json(&json!({
            "agent_type": "claude_code",
            "prompt": "Second task"
        }))
        .await;

    response.assert_status(StatusCode::CONFLICT);
}

#[tokio::test]
async fn test_start_agent_session_400_for_empty_prompt() {
    let (_tmp, server) = create_test_server().await;
    let (_project_id, task_id) = create_project_and_task(&server).await;

    let response = server
        .post(&format!("/api/tasks/{}/sessions/start", task_id))
        .json(&json!({
            "agent_type": "claude_code",
            "prompt": ""
        }))
        .await;

    response.assert_status(StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_start_agent_session_404_for_nonexistent_task() {
    let (_tmp, server) = create_test_server().await;
    let fake_id = Uuid::new_v4();

    let response = server
        .post(&format!("/api/tasks/{}/sessions/start", fake_id))
        .json(&json!({
            "agent_type": "claude_code",
            "prompt": "test"
        }))
        .await;

    response.assert_status(StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_stop_agent_session_returns_200() {
    let (_tmp, server) = create_test_server().await;
    let (_project_id, task_id) = create_project_and_task(&server).await;

    let start_response = server
        .post(&format!("/api/tasks/{}/sessions/start", task_id))
        .json(&json!({
            "agent_type": "claude_code",
            "prompt": "test prompt"
        }))
        .await;
    start_response.assert_status(StatusCode::CREATED);
    let start_body: serde_json::Value = start_response.json();
    let session_id = start_body["session"]["id"].as_str().unwrap();

    let response = server
        .post(&format!(
            "/api/tasks/{}/sessions/{}/stop",
            task_id, session_id
        ))
        .await;

    response.assert_status_ok();
    let session: serde_json::Value = response.json();
    assert_eq!(session["status"], "cancelled");
    assert!(!session["finished_at"].is_null());
}

#[tokio::test]
async fn test_stop_agent_session_404_for_nonrunning_session() {
    let (_tmp, server) = create_test_server().await;
    let (_project_id, task_id) = create_project_and_task(&server).await;
    let fake_session_id = Uuid::new_v4();

    let response = server
        .post(&format!(
            "/api/tasks/{}/sessions/{}/stop",
            task_id, fake_session_id
        ))
        .await;

    response.assert_status(StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_start_session_saves_prompt() {
    let (_tmp, server) = create_test_server().await;
    let (_project_id, task_id) = create_project_and_task(&server).await;

    let response = server
        .post(&format!("/api/tasks/{}/sessions/start", task_id))
        .json(&json!({
            "agent_type": "claude_code",
            "prompt": "Fix the bug in main.rs"
        }))
        .await;

    response.assert_status(StatusCode::CREATED);
    let body: serde_json::Value = response.json();
    assert_eq!(body["session"]["prompt"], "Fix the bug in main.rs");
}
