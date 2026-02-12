mod common;

use axum::http::StatusCode;
use common::create_test_server;

#[tokio::test]
async fn test_health_check_returns_json_with_db_status() {
    let server = create_test_server().await;

    let response = server.get("/health").await;

    response.assert_status(StatusCode::OK);

    let body: serde_json::Value = response.json();
    assert_eq!(body["status"], "ok");
    assert_eq!(body["db"], "connected");
}

#[tokio::test]
async fn test_health_check_returns_version() {
    let server = create_test_server().await;

    let response = server.get("/health").await;
    let body: serde_json::Value = response.json();

    // Version should be a non-empty string matching Cargo.toml version
    let version = body["version"]
        .as_str()
        .expect("version should be a string");
    assert!(!version.is_empty(), "version should not be empty");
    assert_eq!(version, env!("CARGO_PKG_VERSION"));
}

#[tokio::test]
async fn test_health_check_returns_uptime() {
    let server = create_test_server().await;

    let response = server.get("/health").await;
    let body: serde_json::Value = response.json();

    let uptime = body["uptime_seconds"]
        .as_u64()
        .expect("uptime_seconds should be a number");
    // Uptime should be at least 0 (test runs quickly)
    assert!(uptime < 60, "uptime should be reasonable for a test");
}
