mod common;

use axum::http::StatusCode;
use common::{create_auth_test_server, create_test_server};
use serde_json::json;

async fn create_test_project(server: &axum_test::TestServer) -> String {
    let response = server
        .post("/api/projects")
        .json(&json!({
            "name": "Sync Project",
            "description": "Test project for sync"
        }))
        .await;
    response.assert_status(StatusCode::CREATED);
    let body: serde_json::Value = response.json();
    body["id"].as_str().unwrap().to_string()
}

async fn create_github_link(server: &axum_test::TestServer, project_id: &str) {
    let response = server
        .post(&format!("/api/projects/{project_id}/github-link"))
        .json(&json!({
            "repo_owner": "octocat",
            "repo_name": "Hello-World"
        }))
        .await;
    response.assert_status(StatusCode::CREATED);
}

#[tokio::test]
async fn test_sync_github_link_returns_ok() {
    let server = create_test_server().await;
    let project_id = create_test_project(&server).await;
    create_github_link(&server, &project_id).await;

    let response = server
        .post(&format!("/api/projects/{project_id}/github-link/sync"))
        .await;

    response.assert_status(StatusCode::OK);
    let body: serde_json::Value = response.json();
    assert_eq!(body["project_id"], project_id);
    assert!(body["pushed"].is_number());
    assert!(body["pulled"].is_number());
}

#[tokio::test]
async fn test_sync_github_link_returns_not_found_when_no_link() {
    let server = create_test_server().await;
    let project_id = create_test_project(&server).await;

    let response = server
        .post(&format!("/api/projects/{project_id}/github-link/sync"))
        .await;

    response.assert_status(StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_sync_github_link_requires_auth() {
    let server = create_auth_test_server().await;

    let response = server
        .post("/api/projects/00000000-0000-0000-0000-000000000001/github-link/sync")
        .await;

    response.assert_status(StatusCode::UNAUTHORIZED);
}
