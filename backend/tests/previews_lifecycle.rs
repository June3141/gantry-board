mod common;

use axum::http::StatusCode;
use common::create_test_server_with_repo;
use serde_json::json;

#[tokio::test]
async fn test_start_preview_returns_202() {
    let (_tmp, server) = create_test_server_with_repo().await;

    server
        .post("/api/worktrees")
        .json(&json!({ "name": "start-wt" }))
        .await
        .assert_status(StatusCode::CREATED);

    let create_resp = server
        .post("/api/previews")
        .json(&json!({ "worktree_name": "start-wt" }))
        .await;
    create_resp.assert_status(StatusCode::CREATED);
    let preview: serde_json::Value = create_resp.json();
    let id = preview["id"].as_str().unwrap();

    let response = server.post(&format!("/api/previews/{id}/start")).await;

    response.assert_status(StatusCode::ACCEPTED);
}

#[tokio::test]
async fn test_start_nonexistent_preview_returns_404() {
    let (_tmp, server) = create_test_server_with_repo().await;

    let response = server
        .post("/api/previews/00000000-0000-0000-0000-000000000000/start")
        .await;

    response.assert_status(StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_stop_preview_updates_status() {
    let (_tmp, server) = create_test_server_with_repo().await;

    server
        .post("/api/worktrees")
        .json(&json!({ "name": "stop-wt" }))
        .await
        .assert_status(StatusCode::CREATED);

    let create_resp = server
        .post("/api/previews")
        .json(&json!({ "worktree_name": "stop-wt" }))
        .await;
    create_resp.assert_status(StatusCode::CREATED);
    let preview: serde_json::Value = create_resp.json();
    let id = preview["id"].as_str().unwrap();

    let response = server.post(&format!("/api/previews/{id}/stop")).await;

    response.assert_status_ok();
    let stopped: serde_json::Value = response.json();
    assert_eq!(stopped["status"], "stopped");
}

#[tokio::test]
async fn test_stop_nonexistent_preview_returns_404() {
    let (_tmp, server) = create_test_server_with_repo().await;

    let response = server
        .post("/api/previews/00000000-0000-0000-0000-000000000000/stop")
        .await;

    response.assert_status(StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_restart_preview_returns_202() {
    let (_tmp, server) = create_test_server_with_repo().await;

    server
        .post("/api/worktrees")
        .json(&json!({ "name": "restart-wt" }))
        .await
        .assert_status(StatusCode::CREATED);

    let create_resp = server
        .post("/api/previews")
        .json(&json!({ "worktree_name": "restart-wt" }))
        .await;
    create_resp.assert_status(StatusCode::CREATED);
    let preview: serde_json::Value = create_resp.json();
    let id = preview["id"].as_str().unwrap();

    let response = server.post(&format!("/api/previews/{id}/restart")).await;

    response.assert_status(StatusCode::ACCEPTED);
}

#[tokio::test]
async fn test_restart_nonexistent_preview_returns_404() {
    let (_tmp, server) = create_test_server_with_repo().await;

    let response = server
        .post("/api/previews/00000000-0000-0000-0000-000000000000/restart")
        .await;

    response.assert_status(StatusCode::NOT_FOUND);
}
