mod common;

use axum::http::StatusCode;
use common::create_test_server_with_repo;
use serde_json::json;

#[tokio::test]
async fn test_list_worktrees_returns_empty() {
    let (_tmp, server) = create_test_server_with_repo().await;

    let response = server.get("/api/worktrees").await;

    response.assert_status_ok();
    let worktrees: Vec<serde_json::Value> = response.json();
    assert!(worktrees.is_empty());
}

#[tokio::test]
async fn test_create_worktree_returns_created() {
    let (_tmp, server) = create_test_server_with_repo().await;

    let response = server
        .post("/api/worktrees")
        .json(&json!({ "name": "my-feature" }))
        .await;

    response.assert_status(StatusCode::CREATED);
    let wt: serde_json::Value = response.json();
    assert_eq!(wt["name"], "my-feature");
    assert_eq!(wt["branch"], "my-feature");
    assert_eq!(wt["is_valid"], true);
    assert!(wt["path"].is_string());
}

#[tokio::test]
async fn test_create_worktree_with_invalid_name_returns_400() {
    let (_tmp, server) = create_test_server_with_repo().await;

    let response = server
        .post("/api/worktrees")
        .json(&json!({ "name": "../escape" }))
        .await;

    response.assert_status(StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_create_duplicate_worktree_returns_409() {
    let (_tmp, server) = create_test_server_with_repo().await;

    server
        .post("/api/worktrees")
        .json(&json!({ "name": "dup-wt" }))
        .await
        .assert_status(StatusCode::CREATED);

    let response = server
        .post("/api/worktrees")
        .json(&json!({ "name": "dup-wt" }))
        .await;

    response.assert_status(StatusCode::CONFLICT);
}

#[tokio::test]
async fn test_get_worktree_returns_existing() {
    let (_tmp, server) = create_test_server_with_repo().await;

    server
        .post("/api/worktrees")
        .json(&json!({ "name": "get-me" }))
        .await
        .assert_status(StatusCode::CREATED);

    let response = server.get("/api/worktrees/get-me").await;

    response.assert_status_ok();
    let wt: serde_json::Value = response.json();
    assert_eq!(wt["name"], "get-me");
    assert_eq!(wt["is_valid"], true);
}

#[tokio::test]
async fn test_get_worktree_not_found_returns_404() {
    let (_tmp, server) = create_test_server_with_repo().await;

    let response = server.get("/api/worktrees/nonexistent").await;

    response.assert_status(StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_delete_worktree_returns_204() {
    let (_tmp, server) = create_test_server_with_repo().await;

    server
        .post("/api/worktrees")
        .json(&json!({ "name": "del-me" }))
        .await
        .assert_status(StatusCode::CREATED);

    let response = server.delete("/api/worktrees/del-me").await;

    response.assert_status(StatusCode::NO_CONTENT);
}

#[tokio::test]
async fn test_delete_worktree_not_found_returns_404() {
    let (_tmp, server) = create_test_server_with_repo().await;

    let response = server.delete("/api/worktrees/nonexistent").await;

    response.assert_status(StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_list_worktrees_after_create() {
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

    let response = server.get("/api/worktrees").await;

    response.assert_status_ok();
    let worktrees: Vec<serde_json::Value> = response.json();
    assert_eq!(worktrees.len(), 2);
}

#[tokio::test]
async fn test_delete_then_list_removes_worktree() {
    let (_tmp, server) = create_test_server_with_repo().await;

    server
        .post("/api/worktrees")
        .json(&json!({ "name": "temp-wt" }))
        .await
        .assert_status(StatusCode::CREATED);

    server.delete("/api/worktrees/temp-wt").await;

    let response = server.get("/api/worktrees").await;
    let worktrees: Vec<serde_json::Value> = response.json();
    assert!(worktrees.is_empty());
}
