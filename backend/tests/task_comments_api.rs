mod common;

use axum::http::StatusCode;
use axum_test::TestServer;
use common::create_test_server;
use serde_json::json;

async fn create_test_project(server: &TestServer) -> String {
    let response = server
        .post("/api/projects")
        .json(&json!({
            "name": "Test Project",
            "description": "A test project"
        }))
        .await;
    response.assert_status(StatusCode::CREATED);
    let body: serde_json::Value = response.json();
    body["id"].as_str().unwrap().to_string()
}

async fn create_test_task(server: &TestServer, project_id: &str) -> String {
    let response = server
        .post("/api/tasks")
        .json(&json!({
            "project_id": project_id,
            "title": "Test Task"
        }))
        .await;
    response.assert_status(StatusCode::CREATED);
    let body: serde_json::Value = response.json();
    body["id"].as_str().unwrap().to_string()
}

#[tokio::test]
async fn test_create_comment_returns_created() {
    let server = create_test_server().await;
    let project_id = create_test_project(&server).await;
    let task_id = create_test_task(&server, &project_id).await;

    let response = server
        .post(&format!("/api/tasks/{}/comments", task_id))
        .json(&json!({ "content": "Hello, world!" }))
        .await;

    response.assert_status(StatusCode::CREATED);
    let comment: serde_json::Value = response.json();
    assert_eq!(comment["task_id"], task_id);
    assert_eq!(comment["content"], "Hello, world!");
    assert!(comment["id"].as_str().is_some());
    assert!(comment["user_name"].as_str().is_some());
    assert!(comment["created_at"].as_str().is_some());
}

#[tokio::test]
async fn test_list_comments_returns_all() {
    let server = create_test_server().await;
    let project_id = create_test_project(&server).await;
    let task_id = create_test_task(&server, &project_id).await;

    server
        .post(&format!("/api/tasks/{}/comments", task_id))
        .json(&json!({ "content": "First" }))
        .await
        .assert_status(StatusCode::CREATED);

    server
        .post(&format!("/api/tasks/{}/comments", task_id))
        .json(&json!({ "content": "Second" }))
        .await
        .assert_status(StatusCode::CREATED);

    let response = server
        .get(&format!("/api/tasks/{}/comments", task_id))
        .await;

    response.assert_status_ok();
    let comments: Vec<serde_json::Value> = response.json();
    assert_eq!(comments.len(), 2);
    assert_eq!(comments[0]["content"], "First");
    assert_eq!(comments[1]["content"], "Second");
}

#[tokio::test]
async fn test_update_comment_by_author() {
    let server = create_test_server().await;
    let project_id = create_test_project(&server).await;
    let task_id = create_test_task(&server, &project_id).await;

    let create_resp = server
        .post(&format!("/api/tasks/{}/comments", task_id))
        .json(&json!({ "content": "Original" }))
        .await;
    create_resp.assert_status(StatusCode::CREATED);
    let comment: serde_json::Value = create_resp.json();
    let comment_id = comment["id"].as_str().unwrap();

    let response = server
        .patch(&format!("/api/tasks/{}/comments/{}", task_id, comment_id))
        .json(&json!({ "content": "Updated" }))
        .await;

    response.assert_status_ok();
    let updated: serde_json::Value = response.json();
    assert_eq!(updated["content"], "Updated");
}

#[tokio::test]
async fn test_delete_comment_by_author() {
    let server = create_test_server().await;
    let project_id = create_test_project(&server).await;
    let task_id = create_test_task(&server, &project_id).await;

    let create_resp = server
        .post(&format!("/api/tasks/{}/comments", task_id))
        .json(&json!({ "content": "To delete" }))
        .await;
    create_resp.assert_status(StatusCode::CREATED);
    let comment: serde_json::Value = create_resp.json();
    let comment_id = comment["id"].as_str().unwrap();

    let response = server
        .delete(&format!("/api/tasks/{}/comments/{}", task_id, comment_id))
        .await;

    response.assert_status(StatusCode::NO_CONTENT);
}

#[tokio::test]
async fn test_create_comment_on_nonexistent_task_returns_not_found() {
    let server = create_test_server().await;
    let fake_task_id = uuid::Uuid::new_v4();

    let response = server
        .post(&format!("/api/tasks/{}/comments", fake_task_id))
        .json(&json!({ "content": "orphan" }))
        .await;

    response.assert_status(StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_create_comment_empty_content_returns_validation_error() {
    let server = create_test_server().await;
    let project_id = create_test_project(&server).await;
    let task_id = create_test_task(&server, &project_id).await;

    let response = server
        .post(&format!("/api/tasks/{}/comments", task_id))
        .json(&json!({ "content": "" }))
        .await;

    response.assert_status(StatusCode::UNPROCESSABLE_ENTITY);
}
