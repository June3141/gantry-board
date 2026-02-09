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
        auth_disabled: true, // Disable auth for API tests
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

async fn create_test_project(server: &TestServer) -> String {
    let response = server
        .post("/api/projects")
        .json(&json!({
            "name": "Test Project"
        }))
        .await;

    response.assert_status(StatusCode::CREATED);
    let body: serde_json::Value = response.json();
    body["id"].as_str().unwrap().to_string()
}

#[tokio::test]
async fn test_list_tasks_returns_empty_initially() {
    let server = create_test_server().await;
    let project_id = create_test_project(&server).await;

    let response = server
        .get(&format!("/api/tasks?project_id={}", project_id))
        .await;

    response.assert_status_ok();
    let tasks: Vec<serde_json::Value> = response.json();
    assert!(tasks.is_empty());
}

#[tokio::test]
async fn test_list_tasks_returns_not_found_for_nonexistent_project() {
    let server = create_test_server().await;
    let nonexistent_id = Uuid::new_v4();

    let response = server
        .get(&format!("/api/tasks?project_id={}", nonexistent_id))
        .await;

    response.assert_status(StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_create_task_returns_created() {
    let server = create_test_server().await;
    let project_id = create_test_project(&server).await;

    let response = server
        .post("/api/tasks")
        .json(&json!({
            "project_id": project_id,
            "title": "Test Task",
            "description": "A test task"
        }))
        .await;

    response.assert_status(StatusCode::CREATED);
    let task: serde_json::Value = response.json();
    assert_eq!(task["title"], "Test Task");
    assert_eq!(task["description"], "A test task");
    assert_eq!(task["project_id"], project_id);
    assert_eq!(task["status"], "backlog");
    assert_eq!(task["priority"], "medium");
}

#[tokio::test]
async fn test_create_task_validates_title() {
    let server = create_test_server().await;
    let project_id = create_test_project(&server).await;

    let response = server
        .post("/api/tasks")
        .json(&json!({
            "project_id": project_id,
            "title": ""
        }))
        .await;

    response.assert_status(StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_create_task_returns_not_found_for_nonexistent_project() {
    let server = create_test_server().await;
    let nonexistent_id = Uuid::new_v4();

    let response = server
        .post("/api/tasks")
        .json(&json!({
            "project_id": nonexistent_id.to_string(),
            "title": "Orphan Task"
        }))
        .await;

    response.assert_status(StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_get_task_returns_existing() {
    let server = create_test_server().await;
    let project_id = create_test_project(&server).await;

    let create_response = server
        .post("/api/tasks")
        .json(&json!({
            "project_id": project_id,
            "title": "Get Me"
        }))
        .await;
    let created: serde_json::Value = create_response.json();
    let task_id = created["id"].as_str().unwrap();

    let response = server.get(&format!("/api/tasks/{}", task_id)).await;

    response.assert_status_ok();
    let task: serde_json::Value = response.json();
    assert_eq!(task["title"], "Get Me");
}

#[tokio::test]
async fn test_get_task_returns_not_found() {
    let server = create_test_server().await;
    let random_id = Uuid::new_v4();

    let response = server.get(&format!("/api/tasks/{}", random_id)).await;

    response.assert_status(StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_update_task_changes_title() {
    let server = create_test_server().await;
    let project_id = create_test_project(&server).await;

    let create_response = server
        .post("/api/tasks")
        .json(&json!({
            "project_id": project_id,
            "title": "Original"
        }))
        .await;
    let created: serde_json::Value = create_response.json();
    let task_id = created["id"].as_str().unwrap();

    let response = server
        .patch(&format!("/api/tasks/{}", task_id))
        .json(&json!({
            "title": "Updated"
        }))
        .await;

    response.assert_status_ok();
    let task: serde_json::Value = response.json();
    assert_eq!(task["title"], "Updated");
}

#[tokio::test]
async fn test_update_task_changes_status() {
    let server = create_test_server().await;
    let project_id = create_test_project(&server).await;

    let create_response = server
        .post("/api/tasks")
        .json(&json!({
            "project_id": project_id,
            "title": "Task"
        }))
        .await;
    let created: serde_json::Value = create_response.json();
    let task_id = created["id"].as_str().unwrap();

    let response = server
        .patch(&format!("/api/tasks/{}", task_id))
        .json(&json!({
            "status": "in_progress"
        }))
        .await;

    response.assert_status_ok();
    let task: serde_json::Value = response.json();
    assert_eq!(task["status"], "in_progress");
}

#[tokio::test]
async fn test_delete_task_removes_from_db() {
    let server = create_test_server().await;
    let project_id = create_test_project(&server).await;

    let create_response = server
        .post("/api/tasks")
        .json(&json!({
            "project_id": project_id,
            "title": "To Delete"
        }))
        .await;
    let created: serde_json::Value = create_response.json();
    let task_id = created["id"].as_str().unwrap();

    let delete_response = server.delete(&format!("/api/tasks/{}", task_id)).await;
    delete_response.assert_status(StatusCode::NO_CONTENT);

    let get_response = server.get(&format!("/api/tasks/{}", task_id)).await;
    get_response.assert_status(StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_list_tasks_returns_created_tasks() {
    let server = create_test_server().await;
    let project_id = create_test_project(&server).await;

    server
        .post("/api/tasks")
        .json(&json!({
            "project_id": project_id,
            "title": "Task 1"
        }))
        .await;
    server
        .post("/api/tasks")
        .json(&json!({
            "project_id": project_id,
            "title": "Task 2"
        }))
        .await;

    let response = server
        .get(&format!("/api/tasks?project_id={}", project_id))
        .await;

    response.assert_status_ok();
    let tasks: Vec<serde_json::Value> = response.json();
    assert_eq!(tasks.len(), 2);
}

#[tokio::test]
async fn test_update_task_validates_empty_title() {
    let server = create_test_server().await;
    let project_id = create_test_project(&server).await;

    let create_response = server
        .post("/api/tasks")
        .json(&json!({
            "project_id": project_id,
            "title": "Valid Title"
        }))
        .await;
    let created: serde_json::Value = create_response.json();
    let task_id = created["id"].as_str().unwrap();

    let response = server
        .patch(&format!("/api/tasks/{}", task_id))
        .json(&json!({
            "title": ""
        }))
        .await;

    response.assert_status(StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_list_tasks_requires_project_id() {
    let server = create_test_server().await;

    let response = server.get("/api/tasks").await;

    response.assert_status(StatusCode::BAD_REQUEST);
}
