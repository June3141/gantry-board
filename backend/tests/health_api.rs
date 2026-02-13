mod common;

use axum::http::{HeaderValue, StatusCode};
use common::create_test_server;
use uuid::Uuid;

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

#[tokio::test]
async fn test_response_contains_x_request_id_header() {
    let server = create_test_server().await;

    let response = server.get("/health").await;

    let header = response.header("x-request-id");
    let request_id = header
        .to_str()
        .expect("x-request-id should be a valid string");
    // Verify it's a valid UUID
    request_id
        .parse::<Uuid>()
        .expect("x-request-id should be a valid UUID");
}

#[tokio::test]
async fn test_metrics_endpoint_returns_prometheus_format() {
    let server = create_test_server().await;

    // Make a request first so there's at least one metric recorded
    server.get("/health").await;

    let response = server.get("/metrics").await;
    response.assert_status(StatusCode::OK);

    let body = response.text();
    // axum-prometheus provides these standard HTTP metrics
    assert!(
        body.contains("axum_http_requests_total"),
        "metrics should contain axum_http_requests_total"
    );
    assert!(
        body.contains("axum_http_requests_duration_seconds"),
        "metrics should contain axum_http_requests_duration_seconds"
    );
}

#[tokio::test]
async fn test_x_request_id_propagated_from_client() {
    let server = create_test_server().await;
    let client_id = Uuid::new_v4().to_string();

    let response = server
        .get("/health")
        .add_header("x-request-id", HeaderValue::from_str(&client_id).unwrap())
        .await;

    let header = response.header("x-request-id");
    let returned_id = header.to_str().expect("x-request-id should be present");
    assert_eq!(returned_id, client_id);
}
