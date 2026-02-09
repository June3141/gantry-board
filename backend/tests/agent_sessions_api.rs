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
use tempfile::TempDir;
use uuid::Uuid;

async fn create_test_server() -> (TempDir, TestServer) {
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

    // Create a temporary git repo for worktree operations
    let tmp = TempDir::new().expect("Failed to create temp dir");
    let repo_path = tmp.path().join("repo");
    std::fs::create_dir(&repo_path).expect("Failed to create repo dir");
    let repo = git2::Repository::init(&repo_path).expect("Failed to init repo");
    let sig = git2::Signature::now("test", "test@test.com").unwrap();
    let tree_id = repo.index().unwrap().write_tree().unwrap();
    let tree = repo.find_tree(tree_id).unwrap();
    repo.commit(Some("HEAD"), &sig, &sig, "initial", &tree, &[])
        .unwrap();

    let sse_hub = Arc::new(SseHub::default());
    let orchestrator = Arc::new(AgentOrchestrator::new(
        Arc::new(NoopExecutor),
        pool.clone(),
        repo_path,
        Arc::clone(&sse_hub),
    ));
    let state = AppState {
        pool,
        sse_hub,
        config: Arc::new(config),
        orchestrator,
    };

    let app = gantry_board::app(state);
    let server = TestServer::new(app).expect("Failed to create test server");
    (tmp, server)
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
    let (_tmp, server) = create_test_server().await;
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
    let (_tmp, server) = create_test_server().await;
    let fake_id = Uuid::new_v4();

    let response = server
        .post(&format!("/api/tasks/{}/sessions", fake_id))
        .json(&json!({ "agent_type": "claude_code" }))
        .await;

    response.assert_status(StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_list_agent_sessions_returns_empty() {
    let (_tmp, server) = create_test_server().await;
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
    let (_tmp, server) = create_test_server().await;
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
    let (_tmp, server) = create_test_server().await;
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
    let (_tmp, server) = create_test_server().await;
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
    let (_tmp, server) = create_test_server().await;
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
    let (_tmp, server) = create_test_server().await;
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
    let (_tmp, server) = create_test_server().await;
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

// ========== Start/Stop Agent Session Tests ==========

#[tokio::test]
async fn test_start_agent_session_returns_created() {
    let (_tmp, server) = create_test_server().await;
    let (_project_id, task_id) = create_test_task(&server).await;

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
    let (_project_id, task_id) = create_test_task(&server).await;

    // First start succeeds
    server
        .post(&format!("/api/tasks/{}/sessions/start", task_id))
        .json(&json!({
            "agent_type": "claude_code",
            "prompt": "First task"
        }))
        .await
        .assert_status(StatusCode::CREATED);

    // Second start should fail with 409 Conflict
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
    let (_project_id, task_id) = create_test_task(&server).await;

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
    let (_project_id, task_id) = create_test_task(&server).await;

    // Start a session first
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

    // Stop the session
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
    let (_project_id, task_id) = create_test_task(&server).await;
    let fake_session_id = Uuid::new_v4();

    let response = server
        .post(&format!(
            "/api/tasks/{}/sessions/{}/stop",
            task_id, fake_session_id
        ))
        .await;

    response.assert_status(StatusCode::NOT_FOUND);
}
