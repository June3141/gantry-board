mod common;

use axum::http::StatusCode;
use common::{create_auth_test_server, create_test_server, register_user};

// ==========================================================================
// #289: DB maintenance metrics
// ==========================================================================

/// WAL size and DB size metrics should appear in /metrics output.
#[tokio::test]
async fn test_db_metrics_include_wal_and_db_size() {
    let server = create_test_server().await;

    // Warm-up request so prometheus records at least one HTTP metric
    server.get("/health").await.assert_status(StatusCode::OK);

    let response = server.get("/metrics").await;
    response.assert_status(StatusCode::OK);
    let body = response.text();

    assert!(
        body.contains("gantry_db_wal_size_bytes"),
        "metrics should include gantry_db_wal_size_bytes"
    );
    assert!(
        body.contains("gantry_db_size_bytes"),
        "metrics should include gantry_db_size_bytes"
    );
}

// ==========================================================================
// #288: Audit logging
// ==========================================================================

/// GET /api/admin/audit-log should return paginated audit events with expected structure.
#[tokio::test]
async fn test_audit_log_returns_valid_structure() {
    let server = create_auth_test_server().await;
    let (_user_id, cookie) = register_user(&server, "admin@test.com", "Admin").await;

    let response = server
        .get("/api/admin/audit-log")
        .add_header(axum::http::header::COOKIE, &cookie)
        .await;
    response.assert_status(StatusCode::OK);

    let body: serde_json::Value = response.json();
    assert!(body["items"].is_array());
    assert!(body["total"].is_number());
    assert!(body["limit"].is_number());
    assert!(body["offset"].is_number());
}

/// Audit events should be recorded for authentication actions.
/// After register + login, there should be audit entries.
#[tokio::test]
async fn test_audit_log_records_auth_events() {
    let server = create_auth_test_server().await;
    let (_user_id, cookie) = register_user(&server, "admin@test.com", "Admin").await;

    let response = server
        .get("/api/admin/audit-log")
        .add_header(axum::http::header::COOKIE, &cookie)
        .await;
    response.assert_status(StatusCode::OK);

    let body: serde_json::Value = response.json();
    let items = body["items"].as_array().unwrap();
    // At least one event from registration
    assert!(
        !items.is_empty(),
        "should have at least one audit event after registration"
    );
    // First event should be auth-related
    let event_type = items[0]["event_type"].as_str().unwrap();
    assert!(
        event_type.starts_with("auth."),
        "event type should start with 'auth.' but got: {}",
        event_type
    );
}

/// Audit log should support filtering by event_type query parameter.
#[tokio::test]
async fn test_audit_log_filters_by_event_type() {
    let server = create_auth_test_server().await;
    let (_user_id, cookie) = register_user(&server, "admin@test.com", "Admin").await;

    let response = server
        .get("/api/admin/audit-log?event_type=auth.register")
        .add_header(axum::http::header::COOKIE, &cookie)
        .await;
    response.assert_status(StatusCode::OK);

    let body: serde_json::Value = response.json();
    let items = body["items"].as_array().unwrap();
    for item in items {
        assert_eq!(item["event_type"].as_str().unwrap(), "auth.register");
    }
}

/// Audit log should support pagination with limit and offset.
#[tokio::test]
async fn test_audit_log_pagination() {
    let server = create_auth_test_server().await;
    let (_user_id, cookie) = register_user(&server, "admin@test.com", "Admin").await;

    let response = server
        .get("/api/admin/audit-log?limit=1&offset=0")
        .add_header(axum::http::header::COOKIE, &cookie)
        .await;
    response.assert_status(StatusCode::OK);

    let body: serde_json::Value = response.json();
    let items = body["items"].as_array().unwrap();
    assert!(items.len() <= 1, "limit=1 should return at most 1 item");
    assert!(
        body["total"].is_number(),
        "response should include total count"
    );
}

/// Unauthenticated requests to audit log should fail.
#[tokio::test]
async fn test_audit_log_requires_auth() {
    let server = create_auth_test_server().await;

    let response = server.get("/api/admin/audit-log").await;
    assert_ne!(response.status_code(), StatusCode::OK);
}

// ==========================================================================
// #287: Admin status API
// ==========================================================================

/// GET /api/admin/status should return system status.
#[tokio::test]
async fn test_admin_status_returns_system_info() {
    let server = create_auth_test_server().await;
    let (_user_id, cookie) = register_user(&server, "admin@test.com", "Admin").await;

    let response = server
        .get("/api/admin/status")
        .add_header(axum::http::header::COOKIE, &cookie)
        .await;
    response.assert_status(StatusCode::OK);

    let body: serde_json::Value = response.json();

    // System info
    assert!(body["version"].is_string(), "should include version");
    assert!(body["uptime_seconds"].is_number(), "should include uptime");

    // DB stats
    assert!(body["db"].is_object(), "should include db stats");
    assert!(body["db"]["pool_size"].is_number());
    assert!(body["db"]["pool_idle"].is_number());

    // Active sessions
    assert!(
        body["active_sessions"].is_array(),
        "should include active sessions list"
    );

    // Connections
    assert!(
        body["realtime_connections"].is_number(),
        "should include connection count"
    );
}

/// Unauthenticated requests to admin status should fail.
#[tokio::test]
async fn test_admin_status_requires_auth() {
    let server = create_auth_test_server().await;

    let response = server.get("/api/admin/status").await;
    assert_ne!(response.status_code(), StatusCode::OK);
}
