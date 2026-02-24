mod common;

use axum::http::StatusCode;
use common::create_test_server;

/// Test that the TraceLayer is configured with enhanced structured logging.
/// We verify this indirectly by ensuring the response still works correctly
/// after our TraceLayer modifications (i.e., the on_response callback
/// does not break the response flow).
#[tokio::test]
async fn test_health_endpoint_still_responds_after_trace_layer_changes() {
    let server = create_test_server().await;

    let response = server.get("/health").await;
    response.assert_status(StatusCode::OK);

    let body: serde_json::Value = response.json();
    assert_eq!(body["status"], "ok");
}

/// Test that a 404 error response is still correctly formed after trace layer changes.
#[tokio::test]
async fn test_error_response_still_correct_after_trace_layer_changes() {
    let server = create_test_server().await;

    let response = server
        .get("/api/tasks/00000000-0000-0000-0000-000000000000")
        .await;
    response.assert_status(StatusCode::NOT_FOUND);

    let body: serde_json::Value = response.json();
    assert!(body["error"]["code"].is_string());
}

/// Test that the metrics endpoint still works (it should be logged at debug level).
#[tokio::test]
async fn test_metrics_endpoint_responds_correctly() {
    let server = create_test_server().await;

    // Warm-up request so that axum-prometheus records at least one HTTP metric
    server.get("/health").await.assert_status(StatusCode::OK);

    let response = server.get("/metrics").await;
    response.assert_status(StatusCode::OK);

    let body = response.text();
    // The Prometheus output should contain our custom gantry metrics (seeded on startup)
    assert!(body.contains("gantry_"));
}
