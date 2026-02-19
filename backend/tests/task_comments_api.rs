mod common;

use axum::http::StatusCode;
use common::{
    create_project_no_auth, create_task_no_auth, create_test_server_with_pool, SqlitePool,
};
use serde_json::json;

/// Seed a user with nil UUID so auth-disabled mode (user_id = Uuid::nil()) can
/// JOIN against the users table when creating comments.
async fn seed_nil_user(pool: &SqlitePool) {
    sqlx::query(
        "INSERT INTO users (id, email, name, password_hash, created_at, updated_at) \
         VALUES ($1, $2, $3, $4, datetime('now'), datetime('now'))",
    )
    .bind(uuid::Uuid::nil().to_string())
    .bind("test@nil.local")
    .bind("Test User")
    .bind("not-a-real-hash")
    .execute(pool)
    .await
    .unwrap();
}

#[tokio::test]
async fn test_create_comment_returns_created() {
    let (server, pool) = create_test_server_with_pool().await;
    seed_nil_user(&pool).await;
    let project_id = create_project_no_auth(&server, "Test Project").await;
    let task_id = create_task_no_auth(&server, &project_id, "Test Task").await;

    let response = server
        .post(&format!("/api/tasks/{}/comments", task_id))
        .json(&json!({ "content": "Hello, world!" }))
        .await;

    response.assert_status(StatusCode::CREATED);
    let comment: serde_json::Value = response.json();
    assert_eq!(comment["task_id"], task_id);
    assert_eq!(comment["content"], "Hello, world!");
    assert!(comment["id"].as_str().is_some());
    assert_eq!(comment["user_name"], "Test User");
    assert!(comment["created_at"].as_str().is_some());
}

#[tokio::test]
async fn test_list_comments_returns_all() {
    let (server, pool) = create_test_server_with_pool().await;
    seed_nil_user(&pool).await;
    let project_id = create_project_no_auth(&server, "Test Project").await;
    let task_id = create_task_no_auth(&server, &project_id, "Test Task").await;

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
    let (server, pool) = create_test_server_with_pool().await;
    seed_nil_user(&pool).await;
    let project_id = create_project_no_auth(&server, "Test Project").await;
    let task_id = create_task_no_auth(&server, &project_id, "Test Task").await;

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
    let (server, pool) = create_test_server_with_pool().await;
    seed_nil_user(&pool).await;
    let project_id = create_project_no_auth(&server, "Test Project").await;
    let task_id = create_task_no_auth(&server, &project_id, "Test Task").await;

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
    let (server, _pool) = create_test_server_with_pool().await;
    let fake_task_id = uuid::Uuid::new_v4();

    let response = server
        .post(&format!("/api/tasks/{}/comments", fake_task_id))
        .json(&json!({ "content": "orphan" }))
        .await;

    response.assert_status(StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_create_comment_empty_content_returns_validation_error() {
    let (server, pool) = create_test_server_with_pool().await;
    seed_nil_user(&pool).await;
    let project_id = create_project_no_auth(&server, "Test Project").await;
    let task_id = create_task_no_auth(&server, &project_id, "Test Task").await;

    let response = server
        .post(&format!("/api/tasks/{}/comments", task_id))
        .json(&json!({ "content": "" }))
        .await;

    response.assert_status(StatusCode::BAD_REQUEST);
}
