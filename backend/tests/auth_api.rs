use std::net::SocketAddr;
use std::sync::Arc;

use axum::http::{header, StatusCode};
use axum_test::TestServer;
use gantry_board::agent::executor::NoopExecutor;
use gantry_board::agent::orchestrator::AgentOrchestrator;
use gantry_board::config::Config;
use gantry_board::sse::hub::SseHub;
use gantry_board::AppState;
use serde_json::json;
use sqlx::sqlite::SqlitePoolOptions;
use std::path::PathBuf;

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
        auth_disabled: false, // Enable auth for these tests
        ..Default::default()
    };

    let sse_hub = Arc::new(SseHub::default());
    let orchestrator = Arc::new(AgentOrchestrator::new(
        Arc::new(NoopExecutor),
        pool.clone(),
        PathBuf::from("."),
        Arc::clone(&sse_hub),
    ));
    let state = AppState {
        pool,
        sse_hub,
        config: Arc::new(config),
        orchestrator,
    };

    let app = gantry_board::app(state).into_make_service_with_connect_info::<SocketAddr>();
    TestServer::new(app).expect("Failed to create test server")
}

#[tokio::test]
async fn test_register_creates_user_and_returns_session_cookie() {
    let server = create_test_server().await;

    let response = server
        .post("/api/auth/register")
        .json(&json!({
            "email": "test@example.com",
            "name": "Test User",
            "password": "password123"
        }))
        .await;

    response.assert_status(StatusCode::CREATED);

    let body: serde_json::Value = response.json();
    assert_eq!(body["user"]["email"], "test@example.com");
    assert_eq!(body["user"]["name"], "Test User");
    assert!(body["user"]["id"].is_string());

    // Check Set-Cookie header
    let cookies = response.headers().get("set-cookie");
    assert!(cookies.is_some(), "Should set session cookie");
    let cookie_str = cookies.unwrap().to_str().unwrap();
    assert!(cookie_str.contains("gantry_session="));
    assert!(cookie_str.contains("HttpOnly"));
}

#[tokio::test]
async fn test_register_validates_email() {
    let server = create_test_server().await;

    let response = server
        .post("/api/auth/register")
        .json(&json!({
            "email": "not-an-email",
            "name": "Test User",
            "password": "password123"
        }))
        .await;

    response.assert_status(StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_register_validates_password_length() {
    let server = create_test_server().await;

    let response = server
        .post("/api/auth/register")
        .json(&json!({
            "email": "test@example.com",
            "name": "Test User",
            "password": "short"  // Less than 8 characters
        }))
        .await;

    response.assert_status(StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_register_duplicate_email_fails() {
    let server = create_test_server().await;

    let body = json!({
        "email": "test@example.com",
        "name": "Test User",
        "password": "password123"
    });

    // First registration should succeed
    server.post("/api/auth/register").json(&body).await;

    // Second registration should fail
    let response = server.post("/api/auth/register").json(&body).await;

    response.assert_status(StatusCode::CONFLICT);
}

#[tokio::test]
async fn test_login_with_valid_credentials() {
    let server = create_test_server().await;

    // Register first
    server
        .post("/api/auth/register")
        .json(&json!({
            "email": "test@example.com",
            "name": "Test User",
            "password": "password123"
        }))
        .await;

    // Login
    let response = server
        .post("/api/auth/login")
        .json(&json!({
            "email": "test@example.com",
            "password": "password123"
        }))
        .await;

    response.assert_status(StatusCode::OK);

    let body: serde_json::Value = response.json();
    assert_eq!(body["user"]["email"], "test@example.com");

    // Check Set-Cookie header
    let cookies = response.headers().get("set-cookie");
    assert!(cookies.is_some(), "Should set session cookie");
}

#[tokio::test]
async fn test_login_with_wrong_password() {
    let server = create_test_server().await;

    // Register first
    server
        .post("/api/auth/register")
        .json(&json!({
            "email": "test@example.com",
            "name": "Test User",
            "password": "password123"
        }))
        .await;

    // Login with wrong password
    let response = server
        .post("/api/auth/login")
        .json(&json!({
            "email": "test@example.com",
            "password": "wrong_password"
        }))
        .await;

    response.assert_status(StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_login_with_nonexistent_email() {
    let server = create_test_server().await;

    let response = server
        .post("/api/auth/login")
        .json(&json!({
            "email": "nonexistent@example.com",
            "password": "password123"
        }))
        .await;

    response.assert_status(StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_me_without_auth() {
    let server = create_test_server().await;

    let response = server.get("/api/auth/me").await;

    response.assert_status(StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_me_with_auth() {
    let server = create_test_server().await;

    // Register and get session cookie
    let register_response = server
        .post("/api/auth/register")
        .json(&json!({
            "email": "test@example.com",
            "name": "Test User",
            "password": "password123"
        }))
        .await;

    let cookies = register_response
        .headers()
        .get("set-cookie")
        .unwrap()
        .to_str()
        .unwrap()
        .to_string();

    // Extract just the cookie value
    let cookie_value = cookies.split(';').next().unwrap();

    // Call /me with cookie
    let response = server
        .get("/api/auth/me")
        .add_header(header::COOKIE, cookie_value)
        .await;

    response.assert_status(StatusCode::OK);

    let body: serde_json::Value = response.json();
    assert_eq!(body["email"], "test@example.com");
    assert_eq!(body["name"], "Test User");
}

#[tokio::test]
async fn test_logout_clears_session() {
    let server = create_test_server().await;

    // Register and get session cookie
    let register_response = server
        .post("/api/auth/register")
        .json(&json!({
            "email": "test@example.com",
            "name": "Test User",
            "password": "password123"
        }))
        .await;

    let cookies = register_response
        .headers()
        .get("set-cookie")
        .unwrap()
        .to_str()
        .unwrap()
        .to_string();

    let cookie_value = cookies.split(';').next().unwrap();

    // Logout
    let logout_response = server
        .post("/api/auth/logout")
        .add_header(header::COOKIE, cookie_value)
        .await;

    logout_response.assert_status(StatusCode::NO_CONTENT);

    // Check that the cookie is cleared
    let clear_cookie = logout_response.headers().get("set-cookie");
    assert!(clear_cookie.is_some());
    let clear_cookie_str = clear_cookie.unwrap().to_str().unwrap();
    assert!(clear_cookie_str.contains("Max-Age=0"));

    // Try to access /me with the old cookie - should fail
    let me_response = server
        .get("/api/auth/me")
        .add_header(header::COOKIE, cookie_value)
        .await;

    me_response.assert_status(StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_logout_without_auth() {
    let server = create_test_server().await;

    let response = server.post("/api/auth/logout").await;

    response.assert_status(StatusCode::UNAUTHORIZED);
}
