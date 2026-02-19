use axum::http::{header, StatusCode};
use serde_json::json;

use crate::common::{
    add_member, create_auth_test_server as create_test_server, create_project,
    create_task_in_project as create_task, register_user,
};

// =============================================================
// Phase 6: Owner protection rules
// =============================================================

// Why: Removing the last owner would leave the project in an unmanageable state —
// no one could add members, change settings, or delete the project.
#[tokio::test]
async fn test_cannot_remove_last_owner() {
    let server = create_test_server().await;
    let (user_a_id, cookie_a) = register_user(&server, "a@example.com", "User A").await;

    let project_id = create_project(&server, &cookie_a, "Project").await;

    let response = server
        .delete(&format!(
            "/api/projects/{}/members/{}",
            project_id, user_a_id
        ))
        .add_header(header::COOKIE, &cookie_a)
        .await;
    response.assert_status(StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_can_remove_owner_if_another_owner_exists() {
    let server = create_test_server().await;
    let (_user_a_id, cookie_a) = register_user(&server, "a@example.com", "User A").await;
    let (user_b_id, _cookie_b) = register_user(&server, "b@example.com", "User B").await;

    let project_id = create_project(&server, &cookie_a, "Project").await;
    add_member(&server, &cookie_a, &project_id, &user_b_id, "owner").await;

    let response = server
        .delete(&format!(
            "/api/projects/{}/members/{}",
            project_id, user_b_id
        ))
        .add_header(header::COOKIE, &cookie_a)
        .await;
    response.assert_status(StatusCode::NO_CONTENT);
}

// Why: Downgrading the last owner to admin/member has the same effect as removal —
// the project loses its ability to perform owner-only operations.
#[tokio::test]
async fn test_cannot_downgrade_last_owner_role() {
    let server = create_test_server().await;
    let (user_a_id, cookie_a) = register_user(&server, "a@example.com", "User A").await;

    let project_id = create_project(&server, &cookie_a, "Project").await;

    let response = server
        .patch(&format!(
            "/api/projects/{}/members/{}",
            project_id, user_a_id
        ))
        .add_header(header::COOKIE, &cookie_a)
        .json(&json!({ "role": "admin" }))
        .await;
    response.assert_status(StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_can_downgrade_owner_if_another_owner_exists() {
    let server = create_test_server().await;
    let (user_a_id, cookie_a) = register_user(&server, "a@example.com", "User A").await;
    let (user_b_id, _cookie_b) = register_user(&server, "b@example.com", "User B").await;

    let project_id = create_project(&server, &cookie_a, "Project").await;
    add_member(&server, &cookie_a, &project_id, &user_b_id, "owner").await;

    let response = server
        .patch(&format!(
            "/api/projects/{}/members/{}",
            project_id, user_a_id
        ))
        .add_header(header::COOKIE, &cookie_a)
        .json(&json!({ "role": "admin" }))
        .await;
    response.assert_status_ok();
}

// =============================================================
// Phase 7: Session outputs endpoint authorization
// =============================================================

// Why: Agent session outputs may contain secrets (env vars, API keys) from the
// execution environment — non-members must never access them.
#[tokio::test]
async fn test_session_outputs_forbidden_for_non_member() {
    let server = create_test_server().await;
    let (_user_a_id, cookie_a) = register_user(&server, "a@example.com", "User A").await;
    let (_user_b_id, cookie_b) = register_user(&server, "b@example.com", "User B").await;

    let project_id = create_project(&server, &cookie_a, "Project").await;
    let task_id = create_task(&server, &cookie_a, &project_id, "Task").await;

    let create_response = server
        .post(&format!("/api/tasks/{}/sessions", task_id))
        .add_header(header::COOKIE, &cookie_a)
        .json(&json!({ "agent_type": "claude_code" }))
        .await;
    create_response.assert_status(StatusCode::CREATED);
    let session: serde_json::Value = create_response.json();
    let session_id = session["id"].as_str().unwrap();

    let response = server
        .get(&format!(
            "/api/tasks/{}/sessions/{}/outputs",
            task_id, session_id
        ))
        .add_header(header::COOKIE, &cookie_b)
        .await;
    response.assert_status(StatusCode::FORBIDDEN);
}
