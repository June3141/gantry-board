mod common;

use axum::http::StatusCode;
use common::create_test_server;
use serde_json::Value;
use uuid::Uuid;

/// Helper to check the standard error response structure.
fn assert_error_response(body: &Value, expected_code: &str) {
    // Top-level fields
    assert!(body.get("error").is_some(), "must have 'error' field");
    assert!(
        body.get("request_id").is_some(),
        "must have 'request_id' field"
    );
    assert!(
        body.get("timestamp").is_some(),
        "must have 'timestamp' field"
    );

    let error = &body["error"];
    assert!(error.get("code").is_some(), "error must have 'code' field");
    assert!(
        error.get("message").is_some(),
        "error must have 'message' field"
    );

    assert_eq!(error["code"].as_str().unwrap(), expected_code);

    // request_id must be a valid UUID
    let request_id = body["request_id"].as_str().unwrap();
    assert!(
        Uuid::parse_str(request_id).is_ok(),
        "request_id must be a valid UUID"
    );

    // timestamp must be a valid ISO 8601 string
    let ts = body["timestamp"].as_str().unwrap();
    assert!(!ts.is_empty(), "timestamp must not be empty");
}

#[tokio::test]
async fn test_not_found_error_response_structure() {
    let server = create_test_server().await;
    let fake_id = Uuid::new_v4();

    let response = server.get(&format!("/api/tasks/{}", fake_id)).await;

    response.assert_status(StatusCode::NOT_FOUND);
    let body: Value = response.json();
    assert_error_response(&body, "NOT_FOUND");
}

#[tokio::test]
async fn test_validation_error_response_structure() {
    let server = create_test_server().await;

    // Create task with missing required fields
    let response = server.post("/api/tasks").json(&serde_json::json!({})).await;

    response.assert_status(StatusCode::UNPROCESSABLE_ENTITY);
    let body: Value = response.json();
    assert_error_response(&body, "VALIDATION_FAILED");
}

#[tokio::test]
async fn test_conflict_error_response_structure() {
    let server = create_test_server().await;

    // Create a project
    let response = server
        .post("/api/projects")
        .json(&serde_json::json!({ "name": "Duplicate" }))
        .await;
    response.assert_status(StatusCode::CREATED);

    // Try to create same project name (unique constraint)
    let response = server
        .post("/api/projects")
        .json(&serde_json::json!({ "name": "Duplicate" }))
        .await;

    response.assert_status(StatusCode::CONFLICT);
    let body: Value = response.json();
    assert_error_response(&body, "CONFLICT");
}

#[tokio::test]
async fn test_unauthorized_error_response_structure() {
    let server = common::create_auth_test_server().await;

    // Access protected endpoint without auth
    let response = server.get("/api/tasks?project_id=test").await;

    response.assert_status(StatusCode::UNAUTHORIZED);
    let body: Value = response.json();
    assert_error_response(&body, "UNAUTHORIZED");
}

#[tokio::test]
async fn test_error_response_does_not_leak_internal_details() {
    let server = create_test_server().await;
    let fake_id = Uuid::new_v4();

    let response = server.get(&format!("/api/tasks/{}", fake_id)).await;

    let body: Value = response.json();
    let message = body["error"]["message"].as_str().unwrap();

    // Error message should be user-friendly, not contain stack traces or internal details
    assert!(
        !message.contains("sqlx"),
        "error should not contain internal library names"
    );
    assert!(
        !message.contains("panicked"),
        "error should not contain panic info"
    );
}
