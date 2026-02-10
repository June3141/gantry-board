mod common;

use common::create_test_server;
use serde_json::json;

#[tokio::test]
async fn test_list_projects_returns_empty_initially() {
    let server = create_test_server().await;

    let response = server.get("/api/projects").await;

    response.assert_status_ok();
    response.assert_json(&json!([]));
}

#[tokio::test]
async fn test_create_project_returns_created() {
    let server = create_test_server().await;

    let response = server
        .post("/api/projects")
        .json(&json!({
            "name": "Test Project",
            "description": "A test project"
        }))
        .await;

    response.assert_status(axum::http::StatusCode::CREATED);
    let body: serde_json::Value = response.json();
    assert_eq!(body["name"], "Test Project");
    assert_eq!(body["description"], "A test project");
    assert!(body["id"].is_string());
}

#[tokio::test]
async fn test_create_project_validates_name() {
    let server = create_test_server().await;

    let response = server
        .post("/api/projects")
        .json(&json!({
            "name": "",
            "description": null
        }))
        .await;

    response.assert_status(axum::http::StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_get_project_returns_existing() {
    let server = create_test_server().await;

    let create_response = server
        .post("/api/projects")
        .json(&json!({
            "name": "Test Project"
        }))
        .await;
    let created: serde_json::Value = create_response.json();
    let id = created["id"].as_str().unwrap();

    let response = server.get(&format!("/api/projects/{}", id)).await;

    response.assert_status_ok();
    let body: serde_json::Value = response.json();
    assert_eq!(body["name"], "Test Project");
}

#[tokio::test]
async fn test_get_project_returns_not_found() {
    let server = create_test_server().await;

    let response = server
        .get("/api/projects/00000000-0000-0000-0000-000000000000")
        .await;

    response.assert_status(axum::http::StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_update_project_changes_name() {
    let server = create_test_server().await;

    let create_response = server
        .post("/api/projects")
        .json(&json!({
            "name": "Original Name"
        }))
        .await;
    let created: serde_json::Value = create_response.json();
    let id = created["id"].as_str().unwrap();

    let response = server
        .patch(&format!("/api/projects/{}", id))
        .json(&json!({
            "name": "Updated Name"
        }))
        .await;

    response.assert_status_ok();
    let body: serde_json::Value = response.json();
    assert_eq!(body["name"], "Updated Name");
}

#[tokio::test]
async fn test_delete_project_removes_from_db() {
    let server = create_test_server().await;

    let create_response = server
        .post("/api/projects")
        .json(&json!({
            "name": "To Be Deleted"
        }))
        .await;
    let created: serde_json::Value = create_response.json();
    let id = created["id"].as_str().unwrap();

    let delete_response = server.delete(&format!("/api/projects/{}", id)).await;
    delete_response.assert_status(axum::http::StatusCode::NO_CONTENT);

    let get_response = server.get(&format!("/api/projects/{}", id)).await;
    get_response.assert_status(axum::http::StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_list_projects_returns_created_projects() {
    let server = create_test_server().await;

    server
        .post("/api/projects")
        .json(&json!({ "name": "Project 1" }))
        .await;
    server
        .post("/api/projects")
        .json(&json!({ "name": "Project 2" }))
        .await;

    let response = server.get("/api/projects").await;

    response.assert_status_ok();
    let body: Vec<serde_json::Value> = response.json();
    assert_eq!(body.len(), 2);
}
