mod common;

use axum::http::StatusCode;
use common::{create_project_no_auth, create_task_no_auth, create_test_server};
use serde_json::json;
use uuid::Uuid;

#[tokio::test]
async fn test_list_tasks_returns_empty_initially() {
    let server = create_test_server().await;
    let project_id = create_project_no_auth(&server, "Test Project").await;

    let response = server
        .get(&format!("/api/tasks?project_id={}", project_id))
        .await;

    response.assert_status_ok();
    let body: serde_json::Value = response.json();
    let tasks = body["data"].as_array().unwrap();
    assert!(tasks.is_empty());
    assert_eq!(body["total"], 0);
    assert_eq!(body["limit"], 50, "default page size must be 50");
    assert_eq!(body["offset"], 0);
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
    let project_id = create_project_no_auth(&server, "Test Project").await;

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
    assert_eq!(
        task["status"], "backlog",
        "new tasks must default to backlog status"
    );
    assert_eq!(
        task["priority"], "medium",
        "new tasks must default to medium priority"
    );
}

#[tokio::test]
async fn test_create_task_validates_title() {
    let server = create_test_server().await;
    let project_id = create_project_no_auth(&server, "Test Project").await;

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
    let project_id = create_project_no_auth(&server, "Test Project").await;
    let task_id = create_task_no_auth(&server, &project_id, "Get Me").await;

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
    let project_id = create_project_no_auth(&server, "Test Project").await;
    let task_id = create_task_no_auth(&server, &project_id, "Original").await;

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
    let project_id = create_project_no_auth(&server, "Test Project").await;
    let task_id = create_task_no_auth(&server, &project_id, "Task").await;

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
    let project_id = create_project_no_auth(&server, "Test Project").await;
    let task_id = create_task_no_auth(&server, &project_id, "To Delete").await;

    let delete_response = server.delete(&format!("/api/tasks/{}", task_id)).await;
    delete_response.assert_status(StatusCode::NO_CONTENT);

    let get_response = server.get(&format!("/api/tasks/{}", task_id)).await;
    get_response.assert_status(StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_list_tasks_returns_created_tasks() {
    let server = create_test_server().await;
    let project_id = create_project_no_auth(&server, "Test Project").await;
    create_task_no_auth(&server, &project_id, "Task 1").await;
    create_task_no_auth(&server, &project_id, "Task 2").await;

    let response = server
        .get(&format!("/api/tasks?project_id={}", project_id))
        .await;

    response.assert_status_ok();
    let body: serde_json::Value = response.json();
    let tasks = body["data"].as_array().unwrap();
    assert_eq!(tasks.len(), 2);
    assert_eq!(body["total"], 2);
}

#[tokio::test]
async fn test_list_tasks_respects_limit_and_offset() {
    let server = create_test_server().await;
    let project_id = create_project_no_auth(&server, "Test Project").await;
    for i in 0..5 {
        create_task_no_auth(&server, &project_id, &format!("Task {}", i)).await;
    }

    let response = server
        .get(&format!(
            "/api/tasks?project_id={}&limit=2&offset=1",
            project_id
        ))
        .await;

    response.assert_status_ok();
    let body: serde_json::Value = response.json();
    let tasks = body["data"].as_array().unwrap();
    assert_eq!(tasks.len(), 2);
    assert_eq!(body["total"], 5);
    assert_eq!(body["limit"], 2);
    assert_eq!(body["offset"], 1);
}

#[tokio::test]
async fn test_update_task_validates_empty_title() {
    let server = create_test_server().await;
    let project_id = create_project_no_auth(&server, "Test Project").await;
    let task_id = create_task_no_auth(&server, &project_id, "Valid Title").await;

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
