mod common;

use axum::http::StatusCode;
use axum_test::TestServer;
use common::{create_test_server_with_pool, SqlitePool};
use serde_json::json;

/// Seed a user with nil UUID so auth-disabled mode (user_id = Uuid::nil()) can
/// JOIN against the users table when creating messages.
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

async fn create_test_project(server: &TestServer) -> String {
    let response = server
        .post("/api/projects")
        .json(&json!({ "name": "Test Project" }))
        .await;
    response.assert_status(StatusCode::CREATED);
    let body: serde_json::Value = response.json();
    body["id"].as_str().unwrap().to_string()
}

// ========== Create ==========

#[tokio::test]
async fn test_create_message_returns_created() {
    let (server, pool) = create_test_server_with_pool().await;
    seed_nil_user(&pool).await;
    let project_id = create_test_project(&server).await;

    let response = server
        .post(&format!("/api/projects/{}/messages", project_id))
        .json(&json!({ "content": "Hello, team!" }))
        .await;

    response.assert_status(StatusCode::CREATED);
    let msg: serde_json::Value = response.json();
    assert_eq!(msg["project_id"], project_id);
    assert_eq!(msg["content"], "Hello, team!");
    assert!(msg["id"].as_str().is_some());
    assert_eq!(msg["user_name"], "Test User");
    assert!(msg["created_at"].as_str().is_some());
}

#[tokio::test]
async fn test_create_message_empty_content_returns_validation_error() {
    let (server, pool) = create_test_server_with_pool().await;
    seed_nil_user(&pool).await;
    let project_id = create_test_project(&server).await;

    let response = server
        .post(&format!("/api/projects/{}/messages", project_id))
        .json(&json!({ "content": "" }))
        .await;

    response.assert_status(StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_create_message_on_nonexistent_project_returns_not_found() {
    let (server, _pool) = create_test_server_with_pool().await;
    let fake_id = uuid::Uuid::new_v4();

    let response = server
        .post(&format!("/api/projects/{}/messages", fake_id))
        .json(&json!({ "content": "orphan" }))
        .await;

    response.assert_status(StatusCode::NOT_FOUND);
}

// ========== List ==========

#[tokio::test]
async fn test_list_messages_returns_newest_first() {
    let (server, pool) = create_test_server_with_pool().await;
    seed_nil_user(&pool).await;
    let project_id = create_test_project(&server).await;

    // Create 3 messages
    for content in &["First", "Second", "Third"] {
        server
            .post(&format!("/api/projects/{}/messages", project_id))
            .json(&json!({ "content": content }))
            .await
            .assert_status(StatusCode::CREATED);
    }

    let response = server
        .get(&format!("/api/projects/{}/messages", project_id))
        .await;

    response.assert_status_ok();
    let messages: Vec<serde_json::Value> = response.json();
    assert_eq!(messages.len(), 3);
    // Newest first (DESC order)
    assert_eq!(messages[0]["content"], "Third");
    assert_eq!(messages[1]["content"], "Second");
    assert_eq!(messages[2]["content"], "First");
}

#[tokio::test]
async fn test_list_messages_with_cursor_pagination() {
    let (server, pool) = create_test_server_with_pool().await;
    seed_nil_user(&pool).await;
    let project_id = create_test_project(&server).await;

    // Create 5 messages
    for i in 1..=5 {
        server
            .post(&format!("/api/projects/{}/messages", project_id))
            .json(&json!({ "content": format!("Message {}", i) }))
            .await
            .assert_status(StatusCode::CREATED);
        // Small delay to ensure distinct timestamps
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }

    // First page: limit=3
    let response = server
        .get(&format!("/api/projects/{}/messages?limit=3", project_id))
        .await;
    response.assert_status_ok();
    let page1: Vec<serde_json::Value> = response.json();
    assert_eq!(page1.len(), 3);
    assert_eq!(page1[0]["content"], "Message 5");
    assert_eq!(page1[1]["content"], "Message 4");
    assert_eq!(page1[2]["content"], "Message 3");

    // Second page: use the last message's created_at as cursor
    let cursor = page1[2]["created_at"].as_str().unwrap();
    let response = server
        .get(&format!(
            "/api/projects/{}/messages?limit=3&before={}",
            project_id, cursor
        ))
        .await;
    response.assert_status_ok();
    let page2: Vec<serde_json::Value> = response.json();
    assert_eq!(page2.len(), 2);
    assert_eq!(page2[0]["content"], "Message 2");
    assert_eq!(page2[1]["content"], "Message 1");
}

#[tokio::test]
async fn test_list_messages_respects_limit() {
    let (server, pool) = create_test_server_with_pool().await;
    seed_nil_user(&pool).await;
    let project_id = create_test_project(&server).await;

    for i in 1..=10 {
        server
            .post(&format!("/api/projects/{}/messages", project_id))
            .json(&json!({ "content": format!("Msg {}", i) }))
            .await
            .assert_status(StatusCode::CREATED);
    }

    let response = server
        .get(&format!("/api/projects/{}/messages?limit=5", project_id))
        .await;
    response.assert_status_ok();
    let messages: Vec<serde_json::Value> = response.json();
    assert_eq!(messages.len(), 5);
}

// ========== Delete ==========

#[tokio::test]
async fn test_delete_message_by_author() {
    let (server, pool) = create_test_server_with_pool().await;
    seed_nil_user(&pool).await;
    let project_id = create_test_project(&server).await;

    let create_resp = server
        .post(&format!("/api/projects/{}/messages", project_id))
        .json(&json!({ "content": "To delete" }))
        .await;
    create_resp.assert_status(StatusCode::CREATED);
    let msg: serde_json::Value = create_resp.json();
    let message_id = msg["id"].as_str().unwrap();

    let response = server
        .delete(&format!(
            "/api/projects/{}/messages/{}",
            project_id, message_id
        ))
        .await;

    response.assert_status(StatusCode::NO_CONTENT);

    // Verify deleted
    let list_resp = server
        .get(&format!("/api/projects/{}/messages", project_id))
        .await;
    let messages: Vec<serde_json::Value> = list_resp.json();
    assert!(messages.is_empty());
}

#[tokio::test]
async fn test_delete_nonexistent_message_returns_not_found() {
    let (server, pool) = create_test_server_with_pool().await;
    seed_nil_user(&pool).await;
    let project_id = create_test_project(&server).await;
    let fake_id = uuid::Uuid::new_v4();

    let response = server
        .delete(&format!(
            "/api/projects/{}/messages/{}",
            project_id, fake_id
        ))
        .await;

    response.assert_status(StatusCode::NOT_FOUND);
}
