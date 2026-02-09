use std::sync::Arc;

use axum::http::StatusCode;
use axum_test::TestServer;
use gantry_board::agent::executor::NoopExecutor;
use gantry_board::agent::orchestrator::AgentOrchestrator;
use gantry_board::config::Config;
use gantry_board::sse::hub::SseHub;
use gantry_board::AppState;
use serde_json::json;
use sqlx::sqlite::SqlitePoolOptions;
use std::path::PathBuf;
use uuid::Uuid;

async fn create_test_server() -> TestServer {
    let pool = SqlitePoolOptions::new()
        .connect("sqlite::memory:")
        .await
        .expect("Failed to create test database");

    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("Failed to run migrations");

    let config = Config {
        bind_addr: "127.0.0.1:0".to_string(),
        database_url: "sqlite::memory:".to_string(),
        auth_disabled: true,
        ..Default::default()
    };

    let sse_hub = Arc::new(SseHub::default());
    let orchestrator = Arc::new(AgentOrchestrator::new(
        Arc::new(NoopExecutor),
        pool.clone(),
        PathBuf::from("."),
        Arc::clone(&sse_hub),
    ));
    let state = AppState {
        pool,
        sse_hub,
        config: Arc::new(config),
        orchestrator,
    };

    let app = gantry_board::app(state);
    TestServer::new(app).expect("Failed to create test server")
}

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

#[tokio::test]
async fn test_create_agent_session_returns_created() {
    let server = create_test_server().await;
    let (_project_id, task_id) = create_test_task(&server).await;

    let response = server
        .post(&format!("/api/tasks/{}/sessions", task_id))
        .json(&json!({ "agent_type": "claude_code" }))
        .await;

    response.assert_status(StatusCode::CREATED);
    let session: serde_json::Value = response.json();
    assert_eq!(session["task_id"], task_id);
    assert_eq!(session["agent_type"], "claude_code");
    assert_eq!(session["status"], "pending");
    assert!(session["started_at"].is_null());
    assert!(session["finished_at"].is_null());
}

#[tokio::test]
async fn test_create_agent_session_for_nonexistent_task_returns_404() {
    let server = create_test_server().await;
    let fake_id = Uuid::new_v4();

    let response = server
        .post(&format!("/api/tasks/{}/sessions", fake_id))
        .json(&json!({ "agent_type": "claude_code" }))
        .await;

    response.assert_status(StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_list_agent_sessions_returns_empty() {
    let server = create_test_server().await;
    let (_project_id, task_id) = create_test_task(&server).await;

    let response = server
        .get(&format!("/api/tasks/{}/sessions", task_id))
        .await;

    response.assert_status_ok();
    let sessions: Vec<serde_json::Value> = response.json();
    assert!(sessions.is_empty());
}

#[tokio::test]
async fn test_list_agent_sessions_returns_created_sessions() {
    let server = create_test_server().await;
    let (_project_id, task_id) = create_test_task(&server).await;

    server
        .post(&format!("/api/tasks/{}/sessions", task_id))
        .json(&json!({ "agent_type": "claude_code" }))
        .await;
    server
        .post(&format!("/api/tasks/{}/sessions", task_id))
        .json(&json!({ "agent_type": "gemini_cli" }))
        .await;

    let response = server
        .get(&format!("/api/tasks/{}/sessions", task_id))
        .await;

    response.assert_status_ok();
    let sessions: Vec<serde_json::Value> = response.json();
    assert_eq!(sessions.len(), 2);
}

#[tokio::test]
async fn test_get_agent_session_returns_existing() {
    let server = create_test_server().await;
    let (_project_id, task_id) = create_test_task(&server).await;

    let create_response = server
        .post(&format!("/api/tasks/{}/sessions", task_id))
        .json(&json!({ "agent_type": "claude_code" }))
        .await;
    let created: serde_json::Value = create_response.json();
    let session_id = created["id"].as_str().unwrap();

    let response = server
        .get(&format!("/api/tasks/{}/sessions/{}", task_id, session_id))
        .await;

    response.assert_status_ok();
    let session: serde_json::Value = response.json();
    assert_eq!(session["id"], session_id);
    assert_eq!(session["agent_type"], "claude_code");
}

#[tokio::test]
async fn test_get_agent_session_returns_not_found() {
    let server = create_test_server().await;
    let (_project_id, task_id) = create_test_task(&server).await;
    let fake_session_id = Uuid::new_v4();

    let response = server
        .get(&format!(
            "/api/tasks/{}/sessions/{}",
            task_id, fake_session_id
        ))
        .await;

    response.assert_status(StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_update_agent_session_changes_status() {
    let server = create_test_server().await;
    let (_project_id, task_id) = create_test_task(&server).await;

    let create_response = server
        .post(&format!("/api/tasks/{}/sessions", task_id))
        .json(&json!({ "agent_type": "claude_code" }))
        .await;
    let created: serde_json::Value = create_response.json();
    let session_id = created["id"].as_str().unwrap();

    let response = server
        .patch(&format!("/api/tasks/{}/sessions/{}", task_id, session_id))
        .json(&json!({ "status": "running" }))
        .await;

    response.assert_status_ok();
    let session: serde_json::Value = response.json();
    assert_eq!(session["status"], "running");
    assert!(!session["started_at"].is_null());
}

#[tokio::test]
async fn test_get_session_under_wrong_task_returns_404() {
    let server = create_test_server().await;
    let (_project_id, task_a) = create_test_task(&server).await;
    let (_project_id2, task_b) = create_test_task(&server).await;

    let create_response = server
        .post(&format!("/api/tasks/{}/sessions", task_a))
        .json(&json!({ "agent_type": "claude_code" }))
        .await;
    let created: serde_json::Value = create_response.json();
    let session_id = created["id"].as_str().unwrap();

    // Access session of task_a via task_b should 404
    let response = server
        .get(&format!("/api/tasks/{}/sessions/{}", task_b, session_id))
        .await;

    response.assert_status(StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_invalid_status_transition_returns_400() {
    let server = create_test_server().await;
    let (_project_id, task_id) = create_test_task(&server).await;

    let create_response = server
        .post(&format!("/api/tasks/{}/sessions", task_id))
        .json(&json!({ "agent_type": "claude_code" }))
        .await;
    let created: serde_json::Value = create_response.json();
    let session_id = created["id"].as_str().unwrap();

    // Pending -> Completed should fail (must go through Running)
    let response = server
        .patch(&format!("/api/tasks/{}/sessions/{}", task_id, session_id))
        .json(&json!({ "status": "completed" }))
        .await;

    response.assert_status(StatusCode::BAD_REQUEST);
}
