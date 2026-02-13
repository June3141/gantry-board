mod common;

use axum::http::StatusCode;
use common::{create_auth_test_server_with_repo, create_test_server_with_repo};
use serde_json::json;

#[tokio::test]
async fn test_create_preview_returns_201() {
    let (_tmp, server) = create_test_server_with_repo().await;

    // Create a worktree first
    server
        .post("/api/worktrees")
        .json(&json!({ "name": "preview-wt" }))
        .await
        .assert_status(StatusCode::CREATED);

    let response = server
        .post("/api/previews")
        .json(&json!({ "worktree_name": "preview-wt" }))
        .await;

    response.assert_status(StatusCode::CREATED);
    let preview: serde_json::Value = response.json();
    assert_eq!(preview["worktree_name"], "preview-wt");
    assert_eq!(preview["status"], "pending");
    assert!(preview["container_id"].is_null());
    assert!(preview["port"].is_null());
    assert!(preview["preview_url"].is_null());
    assert!(preview["error_message"].is_null());
}

#[tokio::test]
async fn test_create_preview_validates_worktree_exists() {
    let (_tmp, server) = create_test_server_with_repo().await;

    let response = server
        .post("/api/previews")
        .json(&json!({ "worktree_name": "nonexistent-wt" }))
        .await;

    response.assert_status(StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_create_duplicate_preview_returns_409() {
    let (_tmp, server) = create_test_server_with_repo().await;

    server
        .post("/api/worktrees")
        .json(&json!({ "name": "dup-wt" }))
        .await
        .assert_status(StatusCode::CREATED);

    server
        .post("/api/previews")
        .json(&json!({ "worktree_name": "dup-wt" }))
        .await
        .assert_status(StatusCode::CREATED);

    let response = server
        .post("/api/previews")
        .json(&json!({ "worktree_name": "dup-wt" }))
        .await;

    response.assert_status(StatusCode::CONFLICT);
}

#[tokio::test]
async fn test_list_previews_empty() {
    let (_tmp, server) = create_test_server_with_repo().await;

    let response = server.get("/api/previews").await;

    response.assert_status_ok();
    let previews: Vec<serde_json::Value> = response.json();
    assert!(previews.is_empty());
}

#[tokio::test]
async fn test_list_previews_after_create() {
    let (_tmp, server) = create_test_server_with_repo().await;

    server
        .post("/api/worktrees")
        .json(&json!({ "name": "wt-a" }))
        .await
        .assert_status(StatusCode::CREATED);
    server
        .post("/api/worktrees")
        .json(&json!({ "name": "wt-b" }))
        .await
        .assert_status(StatusCode::CREATED);

    server
        .post("/api/previews")
        .json(&json!({ "worktree_name": "wt-a" }))
        .await
        .assert_status(StatusCode::CREATED);
    server
        .post("/api/previews")
        .json(&json!({ "worktree_name": "wt-b" }))
        .await
        .assert_status(StatusCode::CREATED);

    let response = server.get("/api/previews").await;

    response.assert_status_ok();
    let previews: Vec<serde_json::Value> = response.json();
    assert_eq!(previews.len(), 2);
}

#[tokio::test]
async fn test_get_preview_by_id() {
    let (_tmp, server) = create_test_server_with_repo().await;

    server
        .post("/api/worktrees")
        .json(&json!({ "name": "get-wt" }))
        .await
        .assert_status(StatusCode::CREATED);

    let create_response = server
        .post("/api/previews")
        .json(&json!({ "worktree_name": "get-wt" }))
        .await;
    create_response.assert_status(StatusCode::CREATED);
    let created: serde_json::Value = create_response.json();
    let id = created["id"].as_str().unwrap();

    let response = server.get(&format!("/api/previews/{id}")).await;

    response.assert_status_ok();
    let preview: serde_json::Value = response.json();
    assert_eq!(preview["id"], id);
    assert_eq!(preview["worktree_name"], "get-wt");
}

#[tokio::test]
async fn test_get_preview_not_found_returns_404() {
    let (_tmp, server) = create_test_server_with_repo().await;

    let response = server
        .get("/api/previews/00000000-0000-0000-0000-000000000000")
        .await;

    response.assert_status(StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_delete_preview_returns_204() {
    let (_tmp, server) = create_test_server_with_repo().await;

    server
        .post("/api/worktrees")
        .json(&json!({ "name": "del-wt" }))
        .await
        .assert_status(StatusCode::CREATED);

    let create_response = server
        .post("/api/previews")
        .json(&json!({ "worktree_name": "del-wt" }))
        .await;
    create_response.assert_status(StatusCode::CREATED);
    let created: serde_json::Value = create_response.json();
    let id = created["id"].as_str().unwrap();

    let response = server.delete(&format!("/api/previews/{id}")).await;

    response.assert_status(StatusCode::NO_CONTENT);
}

#[tokio::test]
async fn test_delete_preview_not_found_returns_404() {
    let (_tmp, server) = create_test_server_with_repo().await;

    let response = server
        .delete("/api/previews/00000000-0000-0000-0000-000000000000")
        .await;

    response.assert_status(StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_preview_endpoints_require_auth() {
    let (_tmp, server) = create_auth_test_server_with_repo().await;

    server
        .get("/api/previews")
        .await
        .assert_status(StatusCode::UNAUTHORIZED);

    server
        .post("/api/previews")
        .json(&json!({ "worktree_name": "test" }))
        .await
        .assert_status(StatusCode::UNAUTHORIZED);

    server
        .get("/api/previews/00000000-0000-0000-0000-000000000000")
        .await
        .assert_status(StatusCode::UNAUTHORIZED);

    server
        .delete("/api/previews/00000000-0000-0000-0000-000000000000")
        .await
        .assert_status(StatusCode::UNAUTHORIZED);
}
