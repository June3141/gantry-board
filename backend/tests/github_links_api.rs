mod common;

use axum::http::StatusCode;
use common::create_test_server;
use serde_json::json;

async fn create_test_project(server: &axum_test::TestServer) -> String {
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

#[tokio::test]
async fn test_create_github_link_returns_created() {
    let server = create_test_server().await;
    let project_id = create_test_project(&server).await;

    let response = server
        .post(&format!("/api/projects/{project_id}/github-link"))
        .json(&json!({
            "repo_owner": "octocat",
            "repo_name": "Hello-World"
        }))
        .await;

    response.assert_status(StatusCode::CREATED);
    let body: serde_json::Value = response.json();
    assert_eq!(body["project_id"], project_id);
    assert_eq!(body["repo_owner"], "octocat");
    assert_eq!(body["repo_name"], "Hello-World");
    assert!(body["id"].as_str().is_some());
}

#[tokio::test]
async fn test_get_github_link_returns_existing() {
    let server = create_test_server().await;
    let project_id = create_test_project(&server).await;

    server
        .post(&format!("/api/projects/{project_id}/github-link"))
        .json(&json!({
            "repo_owner": "octocat",
            "repo_name": "Hello-World"
        }))
        .await;

    let response = server
        .get(&format!("/api/projects/{project_id}/github-link"))
        .await;

    response.assert_status(StatusCode::OK);
    let body: serde_json::Value = response.json();
    assert_eq!(body["repo_owner"], "octocat");
    assert_eq!(body["repo_name"], "Hello-World");
}

#[tokio::test]
async fn test_delete_github_link_returns_no_content() {
    let server = create_test_server().await;
    let project_id = create_test_project(&server).await;

    server
        .post(&format!("/api/projects/{project_id}/github-link"))
        .json(&json!({
            "repo_owner": "octocat",
            "repo_name": "Hello-World"
        }))
        .await;

    let response = server
        .delete(&format!("/api/projects/{project_id}/github-link"))
        .await;

    response.assert_status(StatusCode::NO_CONTENT);

    // Verify it's gone
    let get_response = server
        .get(&format!("/api/projects/{project_id}/github-link"))
        .await;
    get_response.assert_status(StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_get_github_link_status() {
    let server = create_test_server().await;
    let project_id = create_test_project(&server).await;

    server
        .post(&format!("/api/projects/{project_id}/github-link"))
        .json(&json!({
            "repo_owner": "octocat",
            "repo_name": "Hello-World"
        }))
        .await;

    let response = server
        .get(&format!("/api/projects/{project_id}/github-link/status"))
        .await;

    response.assert_status(StatusCode::OK);
    let body: serde_json::Value = response.json();
    assert_eq!(body["repo_owner"], "octocat");
    assert_eq!(body["repo_name"], "Hello-World");
    // No GitHub client configured in test → connected = false
    assert_eq!(body["connected"], false);
}

#[tokio::test]
async fn test_create_duplicate_link_returns_conflict() {
    let server = create_test_server().await;
    let project_id = create_test_project(&server).await;

    let first = server
        .post(&format!("/api/projects/{project_id}/github-link"))
        .json(&json!({
            "repo_owner": "octocat",
            "repo_name": "Hello-World"
        }))
        .await;
    first.assert_status(StatusCode::CREATED);

    let second = server
        .post(&format!("/api/projects/{project_id}/github-link"))
        .json(&json!({
            "repo_owner": "another",
            "repo_name": "repo"
        }))
        .await;
    second.assert_status(StatusCode::CONFLICT);
}

#[tokio::test]
async fn test_get_nonexistent_link_returns_not_found() {
    let server = create_test_server().await;
    let project_id = create_test_project(&server).await;

    let response = server
        .get(&format!("/api/projects/{project_id}/github-link"))
        .await;

    response.assert_status(StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_delete_nonexistent_link_returns_not_found() {
    let server = create_test_server().await;
    let project_id = create_test_project(&server).await;

    let response = server
        .delete(&format!("/api/projects/{project_id}/github-link"))
        .await;

    response.assert_status(StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_github_link_endpoints_require_auth() {
    let server = common::create_auth_test_server().await;

    let fake_project_id = "00000000-0000-0000-0000-000000000000";

    let post = server
        .post(&format!("/api/projects/{fake_project_id}/github-link"))
        .json(&json!({
            "repo_owner": "octocat",
            "repo_name": "Hello-World"
        }))
        .await;
    post.assert_status(StatusCode::UNAUTHORIZED);

    let get = server
        .get(&format!("/api/projects/{fake_project_id}/github-link"))
        .await;
    get.assert_status(StatusCode::UNAUTHORIZED);

    let delete = server
        .delete(&format!("/api/projects/{fake_project_id}/github-link"))
        .await;
    delete.assert_status(StatusCode::UNAUTHORIZED);

    let status = server
        .get(&format!(
            "/api/projects/{fake_project_id}/github-link/status"
        ))
        .await;
    status.assert_status(StatusCode::UNAUTHORIZED);
}
