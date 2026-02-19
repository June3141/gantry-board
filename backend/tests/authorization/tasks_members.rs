use axum::http::{header, StatusCode};
use serde_json::json;

use crate::common::{
    add_member, create_auth_test_server as create_test_server, create_project,
    create_task_in_project as create_task, register_user,
};

// =============================================================
// Phase 4: Task endpoint authorization
// =============================================================

// Why: All task CRUD endpoints must enforce project membership — without this,
// any authenticated user could read/modify/delete tasks in projects they don't belong to.
#[tokio::test]
async fn test_task_endpoints_forbidden_for_non_member() {
    let server = create_test_server().await;
    let (_user_a_id, cookie_a) = register_user(&server, "a@example.com", "User A").await;
    let (_user_b_id, cookie_b) = register_user(&server, "b@example.com", "User B").await;

    let project_id = create_project(&server, &cookie_a, "Project").await;
    let task_id = create_task(&server, &cookie_a, &project_id, "Task").await;

    server
        .get(&format!("/api/tasks?project_id={}", project_id))
        .add_header(header::COOKIE, &cookie_b)
        .await
        .assert_status(StatusCode::FORBIDDEN);

    server
        .post("/api/tasks")
        .add_header(header::COOKIE, &cookie_b)
        .json(&json!({ "project_id": project_id, "title": "Unauthorized" }))
        .await
        .assert_status(StatusCode::FORBIDDEN);

    server
        .get(&format!("/api/tasks/{}", task_id))
        .add_header(header::COOKIE, &cookie_b)
        .await
        .assert_status(StatusCode::FORBIDDEN);

    server
        .patch(&format!("/api/tasks/{}", task_id))
        .add_header(header::COOKIE, &cookie_b)
        .json(&json!({ "title": "Hacked" }))
        .await
        .assert_status(StatusCode::FORBIDDEN);

    server
        .delete(&format!("/api/tasks/{}", task_id))
        .add_header(header::COOKIE, &cookie_b)
        .await
        .assert_status(StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn test_task_endpoints_allowed_for_member() {
    let server = create_test_server().await;
    let (_user_a_id, cookie_a) = register_user(&server, "a@example.com", "User A").await;
    let (user_b_id, cookie_b) = register_user(&server, "b@example.com", "User B").await;

    let project_id = create_project(&server, &cookie_a, "Project").await;
    add_member(&server, &cookie_a, &project_id, &user_b_id, "member").await;

    server
        .get(&format!("/api/tasks?project_id={}", project_id))
        .add_header(header::COOKIE, &cookie_b)
        .await
        .assert_status_ok();

    server
        .post("/api/tasks")
        .add_header(header::COOKIE, &cookie_b)
        .json(&json!({ "project_id": project_id, "title": "Member Task" }))
        .await
        .assert_status(StatusCode::CREATED);
}

// =============================================================
// Phase 5: Member management authorization
// =============================================================

// Why: Member list reveals who works on a project — non-members must not access
// this information to maintain organizational privacy.
#[tokio::test]
async fn test_list_members_forbidden_for_non_member() {
    let server = create_test_server().await;
    let (_user_a_id, cookie_a) = register_user(&server, "a@example.com", "User A").await;
    let (_user_b_id, cookie_b) = register_user(&server, "b@example.com", "User B").await;

    let project_id = create_project(&server, &cookie_a, "Project").await;

    let response = server
        .get(&format!("/api/projects/{}/members", project_id))
        .add_header(header::COOKIE, &cookie_b)
        .await;
    response.assert_status(StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn test_list_members_allowed_for_member() {
    let server = create_test_server().await;
    let (_user_a_id, cookie_a) = register_user(&server, "a@example.com", "User A").await;
    let (user_b_id, cookie_b) = register_user(&server, "b@example.com", "User B").await;

    let project_id = create_project(&server, &cookie_a, "Project").await;
    add_member(&server, &cookie_a, &project_id, &user_b_id, "member").await;

    let response = server
        .get(&format!("/api/projects/{}/members", project_id))
        .add_header(header::COOKIE, &cookie_b)
        .await;
    response.assert_status_ok();
}

// Why: Only admins/owners should add members — allowing regular members to invite
// would bypass the intended approval hierarchy and could lead to unauthorized access.
#[tokio::test]
async fn test_add_member_forbidden_for_member_role() {
    let server = create_test_server().await;
    let (_user_a_id, cookie_a) = register_user(&server, "a@example.com", "User A").await;
    let (user_b_id, cookie_b) = register_user(&server, "b@example.com", "User B").await;
    let (user_c_id, _cookie_c) = register_user(&server, "c@example.com", "User C").await;

    let project_id = create_project(&server, &cookie_a, "Project").await;
    add_member(&server, &cookie_a, &project_id, &user_b_id, "member").await;

    let response = server
        .post(&format!("/api/projects/{}/members", project_id))
        .add_header(header::COOKIE, &cookie_b)
        .json(&json!({ "user_id": user_c_id, "role": "member" }))
        .await;
    response.assert_status(StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn test_add_member_allowed_for_admin() {
    let server = create_test_server().await;
    let (_user_a_id, cookie_a) = register_user(&server, "a@example.com", "User A").await;
    let (user_b_id, cookie_b) = register_user(&server, "b@example.com", "User B").await;
    let (user_c_id, _cookie_c) = register_user(&server, "c@example.com", "User C").await;

    let project_id = create_project(&server, &cookie_a, "Project").await;
    add_member(&server, &cookie_a, &project_id, &user_b_id, "admin").await;

    let response = server
        .post(&format!("/api/projects/{}/members", project_id))
        .add_header(header::COOKIE, &cookie_b)
        .json(&json!({ "user_id": user_c_id, "role": "member" }))
        .await;
    response.assert_status(StatusCode::CREATED);
}

#[tokio::test]
async fn test_update_member_forbidden_for_member_role() {
    let server = create_test_server().await;
    let (_user_a_id, cookie_a) = register_user(&server, "a@example.com", "User A").await;
    let (user_b_id, cookie_b) = register_user(&server, "b@example.com", "User B").await;
    let (user_c_id, _cookie_c) = register_user(&server, "c@example.com", "User C").await;

    let project_id = create_project(&server, &cookie_a, "Project").await;
    add_member(&server, &cookie_a, &project_id, &user_b_id, "member").await;
    add_member(&server, &cookie_a, &project_id, &user_c_id, "member").await;

    let response = server
        .patch(&format!(
            "/api/projects/{}/members/{}",
            project_id, user_c_id
        ))
        .add_header(header::COOKIE, &cookie_b)
        .json(&json!({ "role": "admin" }))
        .await;
    response.assert_status(StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn test_remove_member_forbidden_for_member_role() {
    let server = create_test_server().await;
    let (_user_a_id, cookie_a) = register_user(&server, "a@example.com", "User A").await;
    let (user_b_id, cookie_b) = register_user(&server, "b@example.com", "User B").await;
    let (user_c_id, _cookie_c) = register_user(&server, "c@example.com", "User C").await;

    let project_id = create_project(&server, &cookie_a, "Project").await;
    add_member(&server, &cookie_a, &project_id, &user_b_id, "member").await;
    add_member(&server, &cookie_a, &project_id, &user_c_id, "member").await;

    let response = server
        .delete(&format!(
            "/api/projects/{}/members/{}",
            project_id, user_c_id
        ))
        .add_header(header::COOKIE, &cookie_b)
        .await;
    response.assert_status(StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn test_remove_member_allowed_for_owner() {
    let server = create_test_server().await;
    let (_user_a_id, cookie_a) = register_user(&server, "a@example.com", "User A").await;
    let (user_b_id, _cookie_b) = register_user(&server, "b@example.com", "User B").await;

    let project_id = create_project(&server, &cookie_a, "Project").await;
    add_member(&server, &cookie_a, &project_id, &user_b_id, "member").await;

    let response = server
        .delete(&format!(
            "/api/projects/{}/members/{}",
            project_id, user_b_id
        ))
        .add_header(header::COOKIE, &cookie_a)
        .await;
    response.assert_status(StatusCode::NO_CONTENT);
}
