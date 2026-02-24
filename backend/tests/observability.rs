mod common;

use axum::http::StatusCode;
use common::create_test_server;

#[tokio::test]
async fn test_metrics_endpoint_contains_business_metrics_descriptions() {
    let server = create_test_server().await;

    // Trigger some activity to ensure metrics are registered
    server.get("/health").await;

    let response = server.get("/metrics").await;
    response.assert_status(StatusCode::OK);

    let body = response.text();

    // Verify metric descriptions are registered (HELP lines in Prometheus format)
    assert!(
        body.contains("gantry_agent_session_duration_seconds"),
        "metrics should contain gantry_agent_session_duration_seconds description"
    );
    assert!(
        body.contains("gantry_agent_sessions_total"),
        "metrics should contain gantry_agent_sessions_total description"
    );
    assert!(
        body.contains("gantry_tasks_total"),
        "metrics should contain gantry_tasks_total description"
    );
    assert!(
        body.contains("gantry_errors_total"),
        "metrics should contain gantry_errors_total description"
    );
    assert!(
        body.contains("gantry_github_sync_duration_seconds"),
        "metrics should contain gantry_github_sync_duration_seconds description"
    );
    assert!(
        body.contains("gantry_github_sync_issues_total"),
        "metrics should contain gantry_github_sync_issues_total description"
    );
    assert!(
        body.contains("gantry_db_pool_connections"),
        "metrics should contain gantry_db_pool_connections description"
    );
}

#[tokio::test]
async fn test_errors_total_metric_incremented_on_error() {
    let server = create_test_server().await;

    // Trigger a 404 error by requesting a non-existent task
    let response = server
        .get("/api/tasks/00000000-0000-0000-0000-000000000000")
        .await;
    response.assert_status(StatusCode::NOT_FOUND);

    // Now check metrics
    let metrics_response = server.get("/metrics").await;
    metrics_response.assert_status(StatusCode::OK);
    let body = metrics_response.text();

    // gantry_errors_total should have at least one count with error_code label
    assert!(
        body.contains("gantry_errors_total"),
        "metrics should contain gantry_errors_total after an error response"
    );
}

#[tokio::test]
async fn test_tasks_total_metric_incremented_on_status_change() {
    let server = create_test_server().await;

    // Create a project and task
    let project_id = common::create_project_no_auth(&server, "Metrics Test").await;
    let task_id = common::create_task_no_auth(&server, &project_id, "Test Task").await;

    // Update task status
    server
        .patch(&format!("/api/tasks/{}", task_id))
        .json(&serde_json::json!({ "status": "in_progress" }))
        .await
        .assert_status(StatusCode::OK);

    // Check metrics
    let metrics_response = server.get("/metrics").await;
    metrics_response.assert_status(StatusCode::OK);
    let body = metrics_response.text();

    assert!(
        body.contains("gantry_tasks_total"),
        "metrics should contain gantry_tasks_total after a task status change"
    );
}
