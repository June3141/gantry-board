mod common;

use axum::http::StatusCode;
use common::{create_auth_test_server, create_test_server};
use serde_json::json;

async fn register_user(server: &axum_test::TestServer, email: &str, name: &str) {
    let password = "Tr0ub4dor&3-correct-horse";
    server
        .post("/api/auth/register")
        .json(&json!({ "email": email, "name": name, "password": password }))
        .await
        .assert_status(StatusCode::CREATED);
}

#[tokio::test]
async fn test_search_users_returns_matching_results() {
    let server = create_test_server().await;
    register_user(&server, "alice@example.com", "Alice Smith").await;
    register_user(&server, "bob@example.com", "Bob Jones").await;

    let response = server.get("/api/users?q=alice").await;

    response.assert_status(StatusCode::OK);
    let body: serde_json::Value = response.json();
    let users = body.as_array().expect("should be array");
    assert_eq!(users.len(), 1);
    assert_eq!(users[0]["name"], "Alice Smith");
}

#[tokio::test]
async fn test_search_users_returns_all_when_no_query() {
    let server = create_test_server().await;
    register_user(&server, "alice@example.com", "Alice").await;
    register_user(&server, "bob@example.com", "Bob").await;

    let response = server.get("/api/users").await;

    response.assert_status(StatusCode::OK);
    let body: serde_json::Value = response.json();
    let users = body.as_array().expect("should be array");
    assert_eq!(users.len(), 2);
}

#[tokio::test]
async fn test_search_users_requires_auth() {
    let server = create_auth_test_server().await;

    let response = server.get("/api/users").await;

    response.assert_status(StatusCode::UNAUTHORIZED);
}
