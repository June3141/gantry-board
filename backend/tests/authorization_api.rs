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

/// Add a member to a project with the given role
async fn add_member(
    server: &TestServer,
    cookie: &str,
    project_id: &str,
    user_id: &str,
    role: &str,
) {
    let response = server
        .post(&format!("/api/projects/{}/members", project_id))
        .add_header(header::COOKIE, cookie)
        .json(&json!({ "user_id": user_id, "role": role }))
        .await;
    response.assert_status(StatusCode::CREATED);
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

// =============================================================
// Phase 3: Project endpoint authorization
// =============================================================

#[tokio::test]
async fn test_list_projects_only_returns_member_projects() {
    let server = create_test_server().await;
    let (_user_a_id, cookie_a) = register_user(&server, "a@example.com", "User A").await;
    let (_user_b_id, cookie_b) = register_user(&server, "b@example.com", "User B").await;

    create_project(&server, &cookie_a, "A's Project 1").await;
    create_project(&server, &cookie_a, "A's Project 2").await;
    create_project(&server, &cookie_b, "B's Project").await;

    // User A should see only their 2 projects
    let response = server
        .get("/api/projects")
        .add_header(header::COOKIE, &cookie_a)
        .await;
    response.assert_status_ok();
    let projects: Vec<serde_json::Value> = response.json();
    assert_eq!(projects.len(), 2);

    // User B should see only their 1 project
    let response = server
        .get("/api/projects")
        .add_header(header::COOKIE, &cookie_b)
        .await;
    response.assert_status_ok();
    let projects: Vec<serde_json::Value> = response.json();
    assert_eq!(projects.len(), 1);
}

#[tokio::test]
async fn test_get_project_forbidden_for_non_member() {
    let server = create_test_server().await;
    let (_user_a_id, cookie_a) = register_user(&server, "a@example.com", "User A").await;
    let (_user_b_id, cookie_b) = register_user(&server, "b@example.com", "User B").await;

    let project_id = create_project(&server, &cookie_a, "A's Project").await;

    // User B cannot access A's project
    let response = server
        .get(&format!("/api/projects/{}", project_id))
        .add_header(header::COOKIE, &cookie_b)
        .await;
    response.assert_status(StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn test_get_project_allowed_for_member() {
    let server = create_test_server().await;
    let (_user_a_id, cookie_a) = register_user(&server, "a@example.com", "User A").await;
    let (user_b_id, cookie_b) = register_user(&server, "b@example.com", "User B").await;

    let project_id = create_project(&server, &cookie_a, "Shared Project").await;

    // Add User B as member
    add_member(&server, &cookie_a, &project_id, &user_b_id, "member").await;

    // User B can now access the project
    let response = server
        .get(&format!("/api/projects/{}", project_id))
        .add_header(header::COOKIE, &cookie_b)
        .await;
    response.assert_status_ok();
}

#[tokio::test]
async fn test_update_project_forbidden_for_member_role() {
    let server = create_test_server().await;
    let (_user_a_id, cookie_a) = register_user(&server, "a@example.com", "User A").await;
    let (user_b_id, cookie_b) = register_user(&server, "b@example.com", "User B").await;

    let project_id = create_project(&server, &cookie_a, "Project").await;
    add_member(&server, &cookie_a, &project_id, &user_b_id, "member").await;

    // Member cannot update project
    let response = server
        .patch(&format!("/api/projects/{}", project_id))
        .add_header(header::COOKIE, &cookie_b)
        .json(&json!({ "name": "Hacked Name" }))
        .await;
    response.assert_status(StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn test_update_project_allowed_for_admin() {
    let server = create_test_server().await;
    let (_user_a_id, cookie_a) = register_user(&server, "a@example.com", "User A").await;
    let (user_b_id, cookie_b) = register_user(&server, "b@example.com", "User B").await;

    let project_id = create_project(&server, &cookie_a, "Project").await;
    add_member(&server, &cookie_a, &project_id, &user_b_id, "admin").await;

    // Admin can update project
    let response = server
        .patch(&format!("/api/projects/{}", project_id))
        .add_header(header::COOKIE, &cookie_b)
        .json(&json!({ "name": "Updated by Admin" }))
        .await;
    response.assert_status_ok();
    let body: serde_json::Value = response.json();
    assert_eq!(body["name"], "Updated by Admin");
}

#[tokio::test]
async fn test_delete_project_forbidden_for_admin() {
    let server = create_test_server().await;
    let (_user_a_id, cookie_a) = register_user(&server, "a@example.com", "User A").await;
    let (user_b_id, cookie_b) = register_user(&server, "b@example.com", "User B").await;

    let project_id = create_project(&server, &cookie_a, "Project").await;
    add_member(&server, &cookie_a, &project_id, &user_b_id, "admin").await;

    // Admin cannot delete project
    let response = server
        .delete(&format!("/api/projects/{}", project_id))
        .add_header(header::COOKIE, &cookie_b)
        .await;
    response.assert_status(StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn test_delete_project_allowed_for_owner() {
    let server = create_test_server().await;
    let (_user_a_id, cookie_a) = register_user(&server, "a@example.com", "User A").await;

    let project_id = create_project(&server, &cookie_a, "Project").await;

    // Owner can delete project
    let response = server
        .delete(&format!("/api/projects/{}", project_id))
        .add_header(header::COOKIE, &cookie_a)
        .await;
    response.assert_status(StatusCode::NO_CONTENT);
}

// =============================================================
// Phase 4: Task endpoint authorization
// =============================================================

/// Create a task in a project and return its ID
async fn create_task(server: &TestServer, cookie: &str, project_id: &str, title: &str) -> String {
    let response = server
        .post("/api/tasks")
        .add_header(header::COOKIE, cookie)
        .json(&json!({
            "project_id": project_id,
            "title": title
        }))
        .await;
    response.assert_status(StatusCode::CREATED);
    let body: serde_json::Value = response.json();
    body["id"].as_str().unwrap().to_string()
}

#[tokio::test]
async fn test_list_tasks_forbidden_for_non_member() {
    let server = create_test_server().await;
    let (_user_a_id, cookie_a) = register_user(&server, "a@example.com", "User A").await;
    let (_user_b_id, cookie_b) = register_user(&server, "b@example.com", "User B").await;

    let project_id = create_project(&server, &cookie_a, "Project").await;

    let response = server
        .get(&format!("/api/tasks?project_id={}", project_id))
        .add_header(header::COOKIE, &cookie_b)
        .await;
    response.assert_status(StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn test_list_tasks_allowed_for_member() {
    let server = create_test_server().await;
    let (_user_a_id, cookie_a) = register_user(&server, "a@example.com", "User A").await;
    let (user_b_id, cookie_b) = register_user(&server, "b@example.com", "User B").await;

    let project_id = create_project(&server, &cookie_a, "Project").await;
    add_member(&server, &cookie_a, &project_id, &user_b_id, "member").await;

    let response = server
        .get(&format!("/api/tasks?project_id={}", project_id))
        .add_header(header::COOKIE, &cookie_b)
        .await;
    response.assert_status_ok();
}

#[tokio::test]
async fn test_create_task_forbidden_for_non_member() {
    let server = create_test_server().await;
    let (_user_a_id, cookie_a) = register_user(&server, "a@example.com", "User A").await;
    let (_user_b_id, cookie_b) = register_user(&server, "b@example.com", "User B").await;

    let project_id = create_project(&server, &cookie_a, "Project").await;

    let response = server
        .post("/api/tasks")
        .add_header(header::COOKIE, &cookie_b)
        .json(&json!({
            "project_id": project_id,
            "title": "Unauthorized Task"
        }))
        .await;
    response.assert_status(StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn test_create_task_allowed_for_member() {
    let server = create_test_server().await;
    let (_user_a_id, cookie_a) = register_user(&server, "a@example.com", "User A").await;
    let (user_b_id, cookie_b) = register_user(&server, "b@example.com", "User B").await;

    let project_id = create_project(&server, &cookie_a, "Project").await;
    add_member(&server, &cookie_a, &project_id, &user_b_id, "member").await;

    let response = server
        .post("/api/tasks")
        .add_header(header::COOKIE, &cookie_b)
        .json(&json!({
            "project_id": project_id,
            "title": "Member Task"
        }))
        .await;
    response.assert_status(StatusCode::CREATED);
}

#[tokio::test]
async fn test_get_task_forbidden_for_non_member() {
    let server = create_test_server().await;
    let (_user_a_id, cookie_a) = register_user(&server, "a@example.com", "User A").await;
    let (_user_b_id, cookie_b) = register_user(&server, "b@example.com", "User B").await;

    let project_id = create_project(&server, &cookie_a, "Project").await;
    let task_id = create_task(&server, &cookie_a, &project_id, "Task").await;

    let response = server
        .get(&format!("/api/tasks/{}", task_id))
        .add_header(header::COOKIE, &cookie_b)
        .await;
    response.assert_status(StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn test_update_task_forbidden_for_non_member() {
    let server = create_test_server().await;
    let (_user_a_id, cookie_a) = register_user(&server, "a@example.com", "User A").await;
    let (_user_b_id, cookie_b) = register_user(&server, "b@example.com", "User B").await;

    let project_id = create_project(&server, &cookie_a, "Project").await;
    let task_id = create_task(&server, &cookie_a, &project_id, "Task").await;

    let response = server
        .patch(&format!("/api/tasks/{}", task_id))
        .add_header(header::COOKIE, &cookie_b)
        .json(&json!({ "title": "Hacked" }))
        .await;
    response.assert_status(StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn test_delete_task_forbidden_for_non_member() {
    let server = create_test_server().await;
    let (_user_a_id, cookie_a) = register_user(&server, "a@example.com", "User A").await;
    let (_user_b_id, cookie_b) = register_user(&server, "b@example.com", "User B").await;

    let project_id = create_project(&server, &cookie_a, "Project").await;
    let task_id = create_task(&server, &cookie_a, &project_id, "Task").await;

    let response = server
        .delete(&format!("/api/tasks/{}", task_id))
        .add_header(header::COOKIE, &cookie_b)
        .await;
    response.assert_status(StatusCode::FORBIDDEN);
}
