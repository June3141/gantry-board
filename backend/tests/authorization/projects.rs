use axum::http::{header, StatusCode};
use serde_json::json;

use crate::common::{
    add_member, create_auth_test_server as create_test_server, create_project, register_user,
};

// =============================================================
// Phase 2: create_project auto-adds creator as Owner
// =============================================================

#[tokio::test]
async fn test_create_project_auto_adds_creator_as_owner() {
    let server = create_test_server().await;
    let (user_id, cookie) = register_user(&server, "owner@example.com", "Owner").await;

    let project_id = create_project(&server, &cookie, "My Project").await;

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

    let response = server
        .get("/api/projects")
        .add_header(header::COOKIE, &cookie_a)
        .await;
    response.assert_status_ok();
    let body: serde_json::Value = response.json();
    let projects = body["data"].as_array().unwrap();
    assert_eq!(projects.len(), 2);

    let response = server
        .get("/api/projects")
        .add_header(header::COOKIE, &cookie_b)
        .await;
    response.assert_status_ok();
    let body: serde_json::Value = response.json();
    let projects = body["data"].as_array().unwrap();
    assert_eq!(projects.len(), 1);
}

#[tokio::test]
async fn test_get_project_forbidden_for_non_member() {
    let server = create_test_server().await;
    let (_user_a_id, cookie_a) = register_user(&server, "a@example.com", "User A").await;
    let (_user_b_id, cookie_b) = register_user(&server, "b@example.com", "User B").await;

    let project_id = create_project(&server, &cookie_a, "A's Project").await;

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
    add_member(&server, &cookie_a, &project_id, &user_b_id, "member").await;

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

    let response = server
        .delete(&format!("/api/projects/{}", project_id))
        .add_header(header::COOKIE, &cookie_a)
        .await;
    response.assert_status(StatusCode::NO_CONTENT);
}
