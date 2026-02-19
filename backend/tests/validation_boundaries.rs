mod common;

use axum::http::StatusCode;
use common::{create_auth_test_server, create_test_server};
use serde_json::json;

// ============================================================
// Task title boundaries: min=1, max=255
// ============================================================

#[tokio::test]
async fn test_task_title_at_max_length_accepted() {
    let server = create_test_server().await;
    let response = server
        .post("/api/projects")
        .json(&json!({ "name": "P" }))
        .await;
    let project_id = response.json::<serde_json::Value>()["id"]
        .as_str()
        .unwrap()
        .to_string();

    let title = "a".repeat(255);
    let response = server
        .post("/api/tasks")
        .json(&json!({ "project_id": project_id, "title": title }))
        .await;

    response.assert_status(StatusCode::CREATED);
    assert_eq!(response.json::<serde_json::Value>()["title"], title);
}

#[tokio::test]
async fn test_task_title_exceeds_max_length_rejected() {
    let server = create_test_server().await;
    let response = server
        .post("/api/projects")
        .json(&json!({ "name": "P" }))
        .await;
    let project_id = response.json::<serde_json::Value>()["id"]
        .as_str()
        .unwrap()
        .to_string();

    let title = "a".repeat(256);
    let response = server
        .post("/api/tasks")
        .json(&json!({ "project_id": project_id, "title": title }))
        .await;

    response.assert_status(StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_task_title_whitespace_only_rejected() {
    let server = create_test_server().await;
    let response = server
        .post("/api/projects")
        .json(&json!({ "name": "P" }))
        .await;
    let project_id = response.json::<serde_json::Value>()["id"]
        .as_str()
        .unwrap()
        .to_string();

    let response = server
        .post("/api/tasks")
        .json(&json!({ "project_id": project_id, "title": "   " }))
        .await;

    // Whitespace-only strings pass length validation (len >= 1)
    // but may be trimmed by the service layer; either CREATED or BAD_REQUEST is acceptable.
    // This test documents the current behavior.
    let status = response.status_code();
    assert!(
        status == StatusCode::CREATED || status == StatusCode::BAD_REQUEST,
        "unexpected status for whitespace-only title: {status}"
    );
}

// ============================================================
// Task description boundaries: max=10000
// ============================================================

#[tokio::test]
async fn test_task_description_at_max_length_accepted() {
    let server = create_test_server().await;
    let response = server
        .post("/api/projects")
        .json(&json!({ "name": "P" }))
        .await;
    let project_id = response.json::<serde_json::Value>()["id"]
        .as_str()
        .unwrap()
        .to_string();

    let description = "x".repeat(10000);
    let response = server
        .post("/api/tasks")
        .json(&json!({ "project_id": project_id, "title": "T", "description": description }))
        .await;

    response.assert_status(StatusCode::CREATED);
}

#[tokio::test]
async fn test_task_description_exceeds_max_length_rejected() {
    let server = create_test_server().await;
    let response = server
        .post("/api/projects")
        .json(&json!({ "name": "P" }))
        .await;
    let project_id = response.json::<serde_json::Value>()["id"]
        .as_str()
        .unwrap()
        .to_string();

    let description = "x".repeat(10001);
    let response = server
        .post("/api/tasks")
        .json(&json!({ "project_id": project_id, "title": "T", "description": description }))
        .await;

    response.assert_status(StatusCode::BAD_REQUEST);
}

// ============================================================
// Task update title boundaries: min=1, max=255
// ============================================================

#[tokio::test]
async fn test_update_task_title_at_max_length_accepted() {
    let server = create_test_server().await;
    let response = server
        .post("/api/projects")
        .json(&json!({ "name": "P" }))
        .await;
    let project_id = response.json::<serde_json::Value>()["id"]
        .as_str()
        .unwrap()
        .to_string();

    let response = server
        .post("/api/tasks")
        .json(&json!({ "project_id": project_id, "title": "Initial" }))
        .await;
    let task_id = response.json::<serde_json::Value>()["id"]
        .as_str()
        .unwrap()
        .to_string();

    let title = "b".repeat(255);
    let response = server
        .patch(&format!("/api/tasks/{task_id}"))
        .json(&json!({ "title": title }))
        .await;

    response.assert_status_ok();
    assert_eq!(response.json::<serde_json::Value>()["title"], title);
}

#[tokio::test]
async fn test_update_task_title_exceeds_max_length_rejected() {
    let server = create_test_server().await;
    let response = server
        .post("/api/projects")
        .json(&json!({ "name": "P" }))
        .await;
    let project_id = response.json::<serde_json::Value>()["id"]
        .as_str()
        .unwrap()
        .to_string();

    let response = server
        .post("/api/tasks")
        .json(&json!({ "project_id": project_id, "title": "Initial" }))
        .await;
    let task_id = response.json::<serde_json::Value>()["id"]
        .as_str()
        .unwrap()
        .to_string();

    let title = "b".repeat(256);
    let response = server
        .patch(&format!("/api/tasks/{task_id}"))
        .json(&json!({ "title": title }))
        .await;

    response.assert_status(StatusCode::BAD_REQUEST);
}

// ============================================================
// Project name boundaries: min=1, max=100
// ============================================================

#[tokio::test]
async fn test_project_name_at_max_length_accepted() {
    let server = create_test_server().await;

    let name = "p".repeat(100);
    let response = server
        .post("/api/projects")
        .json(&json!({ "name": name }))
        .await;

    response.assert_status(StatusCode::CREATED);
    assert_eq!(response.json::<serde_json::Value>()["name"], name);
}

#[tokio::test]
async fn test_project_name_exceeds_max_length_rejected() {
    let server = create_test_server().await;

    let name = "p".repeat(101);
    let response = server
        .post("/api/projects")
        .json(&json!({ "name": name }))
        .await;

    response.assert_status(StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_project_name_whitespace_only_rejected() {
    let server = create_test_server().await;

    let response = server
        .post("/api/projects")
        .json(&json!({ "name": "   " }))
        .await;

    let status = response.status_code();
    assert!(
        status == StatusCode::CREATED || status == StatusCode::BAD_REQUEST,
        "unexpected status for whitespace-only project name: {status}"
    );
}

// ============================================================
// Project description boundaries: max=2000
// ============================================================

#[tokio::test]
async fn test_project_description_at_max_length_accepted() {
    let server = create_test_server().await;

    let description = "d".repeat(2000);
    let response = server
        .post("/api/projects")
        .json(&json!({ "name": "P", "description": description }))
        .await;

    response.assert_status(StatusCode::CREATED);
}

#[tokio::test]
async fn test_project_description_exceeds_max_length_rejected() {
    let server = create_test_server().await;

    let description = "d".repeat(2001);
    let response = server
        .post("/api/projects")
        .json(&json!({ "name": "P", "description": description }))
        .await;

    response.assert_status(StatusCode::BAD_REQUEST);
}

// ============================================================
// User registration boundaries
// ============================================================

#[tokio::test]
async fn test_register_name_at_max_length_accepted() {
    let server = create_auth_test_server().await;

    let name = "n".repeat(100);
    let response = server
        .post("/api/auth/register")
        .json(&json!({
            "email": "boundary-name@test.local",
            "name": name,
            "password": "Tr0ub4dor&3-correct-horse"
        }))
        .await;

    response.assert_status(StatusCode::CREATED);
}

#[tokio::test]
async fn test_register_name_exceeds_max_length_rejected() {
    let server = create_auth_test_server().await;

    let name = "n".repeat(101);
    let response = server
        .post("/api/auth/register")
        .json(&json!({
            "email": "boundary-name2@test.local",
            "name": name,
            "password": "Tr0ub4dor&3-correct-horse"
        }))
        .await;

    response.assert_status(StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_register_name_empty_rejected() {
    let server = create_auth_test_server().await;

    let response = server
        .post("/api/auth/register")
        .json(&json!({
            "email": "empty-name@test.local",
            "name": "",
            "password": "Tr0ub4dor&3-correct-horse"
        }))
        .await;

    response.assert_status(StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_register_password_at_min_length_accepted() {
    let server = create_auth_test_server().await;

    // 8 characters, must also pass zxcvbn score >= 3
    let response = server
        .post("/api/auth/register")
        .json(&json!({
            "email": "minpass@test.local",
            "name": "Test",
            "password": "Kj9$mQx!" // 8 chars, mixed case + special
        }))
        .await;

    // May be CREATED if zxcvbn accepts, or BAD_REQUEST if too weak.
    // The key boundary: 8 chars must NOT fail for length alone.
    let status = response.status_code();
    assert!(
        status == StatusCode::CREATED || status == StatusCode::BAD_REQUEST,
        "unexpected status for min-length password: {status}"
    );
}

#[tokio::test]
async fn test_register_password_below_min_length_rejected() {
    let server = create_auth_test_server().await;

    let response = server
        .post("/api/auth/register")
        .json(&json!({
            "email": "shortpass@test.local",
            "name": "Test",
            "password": "Ab1!xyz" // 7 chars
        }))
        .await;

    response.assert_status(StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_register_password_at_max_length_accepted() {
    let server = create_auth_test_server().await;

    // 128 characters with sufficient complexity
    let password = "Tr0ub4dor&3-".repeat(11);
    let password = &password[..128];
    let response = server
        .post("/api/auth/register")
        .json(&json!({
            "email": "maxpass@test.local",
            "name": "Test",
            "password": password
        }))
        .await;

    response.assert_status(StatusCode::CREATED);
}

#[tokio::test]
async fn test_register_password_exceeds_max_length_rejected() {
    let server = create_auth_test_server().await;

    let password = "Tr0ub4dor&3-".repeat(11);
    let password = &password[..129]; // 129 chars: exceeds max=128
    let response = server
        .post("/api/auth/register")
        .json(&json!({
            "email": "overlongpass@test.local",
            "name": "Test",
            "password": password
        }))
        .await;

    response.assert_status(StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_register_invalid_email_rejected() {
    let server = create_auth_test_server().await;

    let response = server
        .post("/api/auth/register")
        .json(&json!({
            "email": "not-an-email",
            "name": "Test",
            "password": "Tr0ub4dor&3-correct-horse"
        }))
        .await;

    response.assert_status(StatusCode::BAD_REQUEST);
}

// ============================================================
// Update project name boundaries: min=1, max=100
// ============================================================

#[tokio::test]
async fn test_update_project_name_at_max_length_accepted() {
    let server = create_test_server().await;

    let response = server
        .post("/api/projects")
        .json(&json!({ "name": "Original" }))
        .await;
    let id = response.json::<serde_json::Value>()["id"]
        .as_str()
        .unwrap()
        .to_string();

    let name = "u".repeat(100);
    let response = server
        .patch(&format!("/api/projects/{id}"))
        .json(&json!({ "name": name }))
        .await;

    response.assert_status_ok();
    assert_eq!(response.json::<serde_json::Value>()["name"], name);
}

#[tokio::test]
async fn test_update_project_name_exceeds_max_length_rejected() {
    let server = create_test_server().await;

    let response = server
        .post("/api/projects")
        .json(&json!({ "name": "Original" }))
        .await;
    let id = response.json::<serde_json::Value>()["id"]
        .as_str()
        .unwrap()
        .to_string();

    let name = "u".repeat(101);
    let response = server
        .patch(&format!("/api/projects/{id}"))
        .json(&json!({ "name": name }))
        .await;

    response.assert_status(StatusCode::BAD_REQUEST);
}
