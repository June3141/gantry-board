mod common;

use axum::http::{header, StatusCode};
use common::{
    add_member, create_auth_test_server, create_auth_test_server_with_repo, create_project,
    create_test_server, create_test_server_with_repo, register_user,
};
use serde_json::json;

// =============================================================
// Issue #274: HTTP request body size limit (2 MB default)
// =============================================================

/// Regular API endpoints should reject bodies larger than 2 MB with 413 Payload Too Large.
#[tokio::test]
async fn test_body_size_limit_rejects_oversized_payload() {
    let server = create_test_server().await;

    // Build a JSON body > 2 MB
    let big_payload = "x".repeat(3 * 1024 * 1024); // 3 MB
    let body = json!({
        "name": big_payload,
    });

    let response = server.post("/api/projects").json(&body).await;

    response.assert_status(StatusCode::PAYLOAD_TOO_LARGE);
}

/// Bodies within the 2 MB limit should be accepted normally.
#[tokio::test]
async fn test_body_size_limit_accepts_normal_payload() {
    let server = create_test_server().await;

    let response = server
        .post("/api/projects")
        .json(&json!({ "name": "Normal Project" }))
        .await;

    response.assert_status(StatusCode::CREATED);
}

// =============================================================
// Issue #272: Remove/sanitize filesystem path from WorktreeResponse
// =============================================================

/// WorktreeResponse should not expose absolute filesystem paths.
#[tokio::test]
async fn test_worktree_response_does_not_contain_absolute_path() {
    let (_tmp, server) = create_test_server_with_repo().await;

    let response = server
        .post("/api/worktrees")
        .json(&json!({ "name": "sanitize-test" }))
        .await;

    response.assert_status(StatusCode::CREATED);
    let wt: serde_json::Value = response.json();

    // The response should NOT have a "path" field at all,
    // OR it should not start with "/" (no absolute path)
    if let Some(path_val) = wt.get("path") {
        if let Some(path_str) = path_val.as_str() {
            assert!(
                !path_str.starts_with('/'),
                "WorktreeResponse should not contain absolute path, got: {path_str}"
            );
        }
    }
}

/// Verify the list endpoint also doesn't leak absolute paths.
#[tokio::test]
async fn test_worktree_list_does_not_contain_absolute_path() {
    let (_tmp, server) = create_test_server_with_repo().await;

    server
        .post("/api/worktrees")
        .json(&json!({ "name": "list-path-test" }))
        .await
        .assert_status(StatusCode::CREATED);

    let response = server.get("/api/worktrees").await;
    response.assert_status_ok();
    let worktrees: Vec<serde_json::Value> = response.json();

    for wt in &worktrees {
        if let Some(path_val) = wt.get("path") {
            if let Some(path_str) = path_val.as_str() {
                assert!(
                    !path_str.starts_with('/'),
                    "WorktreeResponse in list should not contain absolute path, got: {path_str}"
                );
            }
        }
    }
}

// =============================================================
// Issue #271: Project-level authorization for worktree/preview endpoints
// =============================================================

/// A user who is not a member of any project should be forbidden from
/// accessing worktree endpoints.
#[tokio::test]
async fn test_worktree_endpoints_forbidden_for_non_member() {
    let (_tmp, server) = create_auth_test_server_with_repo().await;

    // Register a user but do NOT add them to any project
    let (_user_id, cookie) = register_user(&server, "outsider@test.com", "Outsider").await;

    // All worktree endpoints should return 403
    server
        .get("/api/worktrees")
        .add_header(header::COOKIE, &cookie)
        .await
        .assert_status(StatusCode::FORBIDDEN);

    server
        .post("/api/worktrees")
        .add_header(header::COOKIE, &cookie)
        .json(&json!({ "name": "test" }))
        .await
        .assert_status(StatusCode::FORBIDDEN);

    server
        .get("/api/worktrees/test")
        .add_header(header::COOKIE, &cookie)
        .await
        .assert_status(StatusCode::FORBIDDEN);

    server
        .delete("/api/worktrees/test")
        .add_header(header::COOKIE, &cookie)
        .await
        .assert_status(StatusCode::FORBIDDEN);
}

/// A user who is a member of a project should be able to access worktree endpoints.
#[tokio::test]
async fn test_worktree_endpoints_allowed_for_project_member() {
    let (_tmp, server) = create_auth_test_server_with_repo().await;

    let (user_id, cookie) = register_user(&server, "member@test.com", "Member").await;

    // Create a project and add the user as a member
    let project_id = create_project(&server, &cookie, "Test Project").await;
    // The creator is automatically an owner, so we already have membership

    // Worktree list should work
    let response = server
        .get("/api/worktrees")
        .add_header(header::COOKIE, &cookie)
        .await;
    response.assert_status_ok();
}

/// A user who is not a member of any project should be forbidden from
/// accessing preview endpoints.
#[tokio::test]
async fn test_preview_endpoints_forbidden_for_non_member() {
    let (_tmp, server) = create_auth_test_server_with_repo().await;

    let (_user_id, cookie) = register_user(&server, "outsider2@test.com", "Outsider2").await;

    server
        .get("/api/previews")
        .add_header(header::COOKIE, &cookie)
        .await
        .assert_status(StatusCode::FORBIDDEN);

    server
        .post("/api/previews")
        .add_header(header::COOKIE, &cookie)
        .json(&json!({ "worktree_name": "test" }))
        .await
        .assert_status(StatusCode::FORBIDDEN);

    server
        .get("/api/previews/00000000-0000-0000-0000-000000000001")
        .add_header(header::COOKIE, &cookie)
        .await
        .assert_status(StatusCode::FORBIDDEN);

    server
        .delete("/api/previews/00000000-0000-0000-0000-000000000001")
        .add_header(header::COOKIE, &cookie)
        .await
        .assert_status(StatusCode::FORBIDDEN);
}
