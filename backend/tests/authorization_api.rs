use std::sync::Arc;

use axum::http::{header, StatusCode};
use axum_test::TestServer;
use gantry_board::config::Config;
use gantry_board::sse::hub::SseHub;
use gantry_board::AppState;
use serde_json::json;
use sqlx::sqlite::SqlitePoolOptions;

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
        auth_disabled: false, // Enable auth for authorization tests
        ..Default::default()
    };

    let state = AppState {
        pool,
        sse_hub: Arc::new(SseHub::default()),
        config: Arc::new(config),
    };

    let app = gantry_board::app(state);
    TestServer::new(app).expect("Failed to create test server")
}

/// Register a user and return (user_id, session_cookie_header_value)
async fn register_user(server: &TestServer, email: &str, name: &str) -> (String, String) {
    let response = server
        .post("/api/auth/register")
        .json(&json!({
            "email": email,
            "name": name,
            "password": "password123"
        }))
        .await;
    response.assert_status(StatusCode::CREATED);

    let body: serde_json::Value = response.json();
    let user_id = body["user"]["id"].as_str().unwrap().to_string();

    let cookies = response
        .headers()
        .get("set-cookie")
        .unwrap()
        .to_str()
        .unwrap();
    let cookie_value = cookies.split(';').next().unwrap().to_string();

    (user_id, cookie_value)
}

/// Create a project and return its ID
async fn create_project(server: &TestServer, cookie: &str, name: &str) -> String {
    let response = server
        .post("/api/projects")
        .add_header(header::COOKIE, cookie)
        .json(&json!({ "name": name }))
        .await;
    response.assert_status(StatusCode::CREATED);
    let body: serde_json::Value = response.json();
    body["id"].as_str().unwrap().to_string()
}

// =============================================================
// Phase 2: create_project auto-adds creator as Owner
// =============================================================

#[tokio::test]
async fn test_create_project_auto_adds_creator_as_owner() {
    let server = create_test_server().await;
    let (user_id, cookie) = register_user(&server, "owner@example.com", "Owner").await;

    let project_id = create_project(&server, &cookie, "My Project").await;

    // Verify creator is in members list as owner
    let response = server
        .get(&format!("/api/projects/{}/members", project_id))
        .add_header(header::COOKIE, &cookie)
        .await;
    response.assert_status_ok();

    let members: Vec<serde_json::Value> = response.json();
    assert_eq!(members.len(), 1);
    assert_eq!(members[0]["user_id"], user_id);
    assert_eq!(members[0]["role"], "owner");
}

#[tokio::test]
async fn test_create_project_creator_can_access_project() {
    let server = create_test_server().await;
    let (_user_id, cookie) = register_user(&server, "owner@example.com", "Owner").await;

    let project_id = create_project(&server, &cookie, "My Project").await;

    let response = server
        .get(&format!("/api/projects/{}", project_id))
        .add_header(header::COOKIE, &cookie)
        .await;
    response.assert_status_ok();

    let body: serde_json::Value = response.json();
    assert_eq!(body["name"], "My Project");
}
