use std::sync::Arc;

use axum::http::StatusCode;
use axum_test::TestServer;
use gantry_board::config::Config;
use gantry_board::sse::hub::SseHub;
use gantry_board::AppState;
use serde_json::json;
use sqlx::sqlite::SqlitePoolOptions;
use uuid::Uuid;

async fn create_test_server() -> TestServer {
    let pool = SqlitePoolOptions::new()
        .connect("sqlite::memory:")
        .await
        .expect("Failed to create test database");

    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("Failed to run migrations");

    let config = Config {
        bind_addr: "127.0.0.1:0".to_string(),
        database_url: "sqlite::memory:".to_string(),
    };

    let state = AppState {
        pool,
        sse_hub: Arc::new(SseHub::default()),
        config: Arc::new(config),
    };

    let app = gantry_board::app(state);
    TestServer::new(app).expect("Failed to create test server")
}

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

#[tokio::test]
async fn test_list_members_returns_empty_initially() {
    let server = create_test_server().await;
    let project_id = create_test_project(&server).await;

    let response = server
        .get(&format!("/api/projects/{}/members", project_id))
        .await;

    response.assert_status_ok();
    let members: Vec<serde_json::Value> = response.json();
    assert!(members.is_empty());
}

#[tokio::test]
async fn test_add_member_returns_created() {
    let server = create_test_server().await;
    let project_id = create_test_project(&server).await;
    let user_id = Uuid::new_v4();

    let response = server
        .post(&format!("/api/projects/{}/members", project_id))
        .json(&json!({
            "user_id": user_id.to_string(),
            "role": "member"
        }))
        .await;

    response.assert_status(StatusCode::CREATED);
    let member: serde_json::Value = response.json();
    assert_eq!(member["project_id"], project_id);
    assert_eq!(member["user_id"], user_id.to_string());
    assert_eq!(member["role"], "member");
}

#[tokio::test]
async fn test_get_member_returns_existing() {
    let server = create_test_server().await;
    let project_id = create_test_project(&server).await;
    let user_id = Uuid::new_v4();

    server
        .post(&format!("/api/projects/{}/members", project_id))
        .json(&json!({
            "user_id": user_id.to_string(),
            "role": "admin"
        }))
        .await
        .assert_status(StatusCode::CREATED);

    let response = server
        .get(&format!("/api/projects/{}/members/{}", project_id, user_id))
        .await;

    response.assert_status_ok();
    let member: serde_json::Value = response.json();
    assert_eq!(member["user_id"], user_id.to_string());
    assert_eq!(member["role"], "admin");
}

#[tokio::test]
async fn test_get_member_returns_not_found() {
    let server = create_test_server().await;
    let project_id = create_test_project(&server).await;
    let random_user_id = Uuid::new_v4();

    let response = server
        .get(&format!(
            "/api/projects/{}/members/{}",
            project_id, random_user_id
        ))
        .await;

    response.assert_status(StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_update_member_changes_role() {
    let server = create_test_server().await;
    let project_id = create_test_project(&server).await;
    let user_id = Uuid::new_v4();

    server
        .post(&format!("/api/projects/{}/members", project_id))
        .json(&json!({
            "user_id": user_id.to_string(),
            "role": "member"
        }))
        .await
        .assert_status(StatusCode::CREATED);

    let response = server
        .patch(&format!("/api/projects/{}/members/{}", project_id, user_id))
        .json(&json!({
            "role": "admin"
        }))
        .await;

    response.assert_status_ok();
    let member: serde_json::Value = response.json();
    assert_eq!(member["role"], "admin");
}

#[tokio::test]
async fn test_remove_member_returns_no_content() {
    let server = create_test_server().await;
    let project_id = create_test_project(&server).await;
    let user_id = Uuid::new_v4();

    server
        .post(&format!("/api/projects/{}/members", project_id))
        .json(&json!({
            "user_id": user_id.to_string(),
            "role": "member"
        }))
        .await
        .assert_status(StatusCode::CREATED);

    let response = server
        .delete(&format!("/api/projects/{}/members/{}", project_id, user_id))
        .await;

    response.assert_status(StatusCode::NO_CONTENT);

    // Verify member is gone
    let get_response = server
        .get(&format!("/api/projects/{}/members/{}", project_id, user_id))
        .await;
    get_response.assert_status(StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_list_members_returns_all_members() {
    let server = create_test_server().await;
    let project_id = create_test_project(&server).await;
    let user1 = Uuid::new_v4();
    let user2 = Uuid::new_v4();

    server
        .post(&format!("/api/projects/{}/members", project_id))
        .json(&json!({
            "user_id": user1.to_string(),
            "role": "owner"
        }))
        .await
        .assert_status(StatusCode::CREATED);

    server
        .post(&format!("/api/projects/{}/members", project_id))
        .json(&json!({
            "user_id": user2.to_string(),
            "role": "member"
        }))
        .await
        .assert_status(StatusCode::CREATED);

    let response = server
        .get(&format!("/api/projects/{}/members", project_id))
        .await;

    response.assert_status_ok();
    let members: Vec<serde_json::Value> = response.json();
    assert_eq!(members.len(), 2);
}

#[tokio::test]
async fn test_list_members_returns_not_found_for_nonexistent_project() {
    let server = create_test_server().await;
    let nonexistent_project_id = Uuid::new_v4();

    let response = server
        .get(&format!("/api/projects/{}/members", nonexistent_project_id))
        .await;

    response.assert_status(StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_add_member_returns_not_found_for_nonexistent_project() {
    let server = create_test_server().await;
    let nonexistent_project_id = Uuid::new_v4();
    let user_id = Uuid::new_v4();

    let response = server
        .post(&format!("/api/projects/{}/members", nonexistent_project_id))
        .json(&json!({
            "user_id": user_id.to_string(),
            "role": "member"
        }))
        .await;

    response.assert_status(StatusCode::NOT_FOUND);
}
