mod common;

use axum::http::header;
use axum::http::StatusCode;
use common::{create_auth_test_server, register_user};

// ========== Create Invitation ==========

#[tokio::test]
async fn test_create_invitation_returns_created() {
    let server = create_auth_test_server().await;
    let (_user_id, cookie) = register_user(&server, "owner@test.com", "Owner").await;

    // Create project (owner is auto-added as owner)
    let resp = server
        .post("/api/projects")
        .add_header(header::COOKIE, &cookie)
        .json(&serde_json::json!({ "name": "Test Project" }))
        .await;
    resp.assert_status(StatusCode::CREATED);
    let project_id = resp.json::<serde_json::Value>()["id"]
        .as_str()
        .unwrap()
        .to_string();

    // Create invitation
    let resp = server
        .post(&format!("/api/projects/{}/invitations", project_id))
        .add_header(header::COOKIE, &cookie)
        .json(&serde_json::json!({}))
        .await;

    resp.assert_status(StatusCode::CREATED);
    let body: serde_json::Value = resp.json();
    assert!(body["token"].as_str().is_some());
    assert!(body["invite_url"].as_str().is_some());
    assert!(body["invitation"]["id"].as_str().is_some());
    assert_eq!(body["invitation"]["role"], "member");
    assert_eq!(body["invitation"]["project_name"], "Test Project");
}

#[tokio::test]
async fn test_create_invitation_with_role() {
    let server = create_auth_test_server().await;
    let (_user_id, cookie) = register_user(&server, "owner@test.com", "Owner").await;

    let resp = server
        .post("/api/projects")
        .add_header(header::COOKIE, &cookie)
        .json(&serde_json::json!({ "name": "Test Project" }))
        .await;
    let project_id = resp.json::<serde_json::Value>()["id"]
        .as_str()
        .unwrap()
        .to_string();

    let resp = server
        .post(&format!("/api/projects/{}/invitations", project_id))
        .add_header(header::COOKIE, &cookie)
        .json(&serde_json::json!({ "role": "admin" }))
        .await;

    resp.assert_status(StatusCode::CREATED);
    let body: serde_json::Value = resp.json();
    assert_eq!(body["invitation"]["role"], "admin");
}

#[tokio::test]
async fn test_create_invitation_forbidden_for_member() {
    let server = create_auth_test_server().await;
    let (_owner_id, owner_cookie) = register_user(&server, "owner@test.com", "Owner").await;
    let (member_id, member_cookie) = register_user(&server, "member@test.com", "Member").await;

    let resp = server
        .post("/api/projects")
        .add_header(header::COOKIE, &owner_cookie)
        .json(&serde_json::json!({ "name": "Test Project" }))
        .await;
    let project_id = resp.json::<serde_json::Value>()["id"]
        .as_str()
        .unwrap()
        .to_string();

    // Add member with member role
    server
        .post(&format!("/api/projects/{}/members", project_id))
        .add_header(header::COOKIE, &owner_cookie)
        .json(&serde_json::json!({ "user_id": member_id, "role": "member" }))
        .await
        .assert_status(StatusCode::CREATED);

    // Member tries to create invitation
    let resp = server
        .post(&format!("/api/projects/{}/invitations", project_id))
        .add_header(header::COOKIE, &member_cookie)
        .json(&serde_json::json!({}))
        .await;

    resp.assert_status(StatusCode::FORBIDDEN);
}

// ========== List Invitations ==========

#[tokio::test]
async fn test_list_invitations() {
    let server = create_auth_test_server().await;
    let (_user_id, cookie) = register_user(&server, "owner@test.com", "Owner").await;

    let resp = server
        .post("/api/projects")
        .add_header(header::COOKIE, &cookie)
        .json(&serde_json::json!({ "name": "Test Project" }))
        .await;
    let project_id = resp.json::<serde_json::Value>()["id"]
        .as_str()
        .unwrap()
        .to_string();

    // Create 2 invitations
    for _ in 0..2 {
        server
            .post(&format!("/api/projects/{}/invitations", project_id))
            .add_header(header::COOKIE, &cookie)
            .json(&serde_json::json!({}))
            .await
            .assert_status(StatusCode::CREATED);
    }

    let resp = server
        .get(&format!("/api/projects/{}/invitations", project_id))
        .add_header(header::COOKIE, &cookie)
        .await;

    resp.assert_status_ok();
    let invitations: Vec<serde_json::Value> = resp.json();
    assert_eq!(invitations.len(), 2);
}

// ========== Delete Invitation ==========

#[tokio::test]
async fn test_delete_invitation() {
    let server = create_auth_test_server().await;
    let (_user_id, cookie) = register_user(&server, "owner@test.com", "Owner").await;

    let resp = server
        .post("/api/projects")
        .add_header(header::COOKIE, &cookie)
        .json(&serde_json::json!({ "name": "Test Project" }))
        .await;
    let project_id = resp.json::<serde_json::Value>()["id"]
        .as_str()
        .unwrap()
        .to_string();

    let resp = server
        .post(&format!("/api/projects/{}/invitations", project_id))
        .add_header(header::COOKIE, &cookie)
        .json(&serde_json::json!({}))
        .await;
    let invitation_id = resp.json::<serde_json::Value>()["invitation"]["id"]
        .as_str()
        .unwrap()
        .to_string();

    let resp = server
        .delete(&format!(
            "/api/projects/{}/invitations/{}",
            project_id, invitation_id
        ))
        .add_header(header::COOKIE, &cookie)
        .await;

    resp.assert_status(StatusCode::NO_CONTENT);

    // Verify deleted
    let resp = server
        .get(&format!("/api/projects/{}/invitations", project_id))
        .add_header(header::COOKIE, &cookie)
        .await;
    let invitations: Vec<serde_json::Value> = resp.json();
    assert!(invitations.is_empty());
}

// ========== Get by Token ==========

#[tokio::test]
async fn test_get_invitation_by_token() {
    let server = create_auth_test_server().await;
    let (_user_id, cookie) = register_user(&server, "owner@test.com", "Owner").await;

    let resp = server
        .post("/api/projects")
        .add_header(header::COOKIE, &cookie)
        .json(&serde_json::json!({ "name": "Test Project" }))
        .await;
    let project_id = resp.json::<serde_json::Value>()["id"]
        .as_str()
        .unwrap()
        .to_string();

    let resp = server
        .post(&format!("/api/projects/{}/invitations", project_id))
        .add_header(header::COOKIE, &cookie)
        .json(&serde_json::json!({}))
        .await;
    let token = resp.json::<serde_json::Value>()["token"]
        .as_str()
        .unwrap()
        .to_string();

    // Get invitation info by token (no auth needed)
    let resp = server.get(&format!("/api/invitations/{}", token)).await;

    resp.assert_status_ok();
    let info: serde_json::Value = resp.json();
    assert_eq!(info["project_name"], "Test Project");
    assert_eq!(info["invited_by_name"], "Owner");
    assert_eq!(info["role"], "member");
    assert_eq!(info["expired"], false);
    assert_eq!(info["accepted"], false);
}

#[tokio::test]
async fn test_get_invitation_by_invalid_token_returns_not_found() {
    let server = create_auth_test_server().await;

    let resp = server.get("/api/invitations/invalid_token_here").await;

    resp.assert_status(StatusCode::NOT_FOUND);
}

// ========== Accept Invitation ==========

#[tokio::test]
async fn test_accept_invitation() {
    let server = create_auth_test_server().await;
    let (_owner_id, owner_cookie) = register_user(&server, "owner@test.com", "Owner").await;
    let (_invitee_id, invitee_cookie) = register_user(&server, "invitee@test.com", "Invitee").await;

    let resp = server
        .post("/api/projects")
        .add_header(header::COOKIE, &owner_cookie)
        .json(&serde_json::json!({ "name": "Test Project" }))
        .await;
    let project_id = resp.json::<serde_json::Value>()["id"]
        .as_str()
        .unwrap()
        .to_string();

    let resp = server
        .post(&format!("/api/projects/{}/invitations", project_id))
        .add_header(header::COOKIE, &owner_cookie)
        .json(&serde_json::json!({}))
        .await;
    let token = resp.json::<serde_json::Value>()["token"]
        .as_str()
        .unwrap()
        .to_string();

    // Accept invitation
    let resp = server
        .post(&format!("/api/invitations/{}/accept", token))
        .add_header(header::COOKIE, &invitee_cookie)
        .await;

    resp.assert_status_ok();
    let invitation: serde_json::Value = resp.json();
    assert!(invitation["accepted_at"].as_str().is_some());

    // Verify user is now a member
    let resp = server
        .get(&format!("/api/projects/{}/members", project_id))
        .add_header(header::COOKIE, &owner_cookie)
        .await;
    let members: Vec<serde_json::Value> = resp.json();
    assert_eq!(members.len(), 2); // owner + invitee
}

#[tokio::test]
async fn test_accept_invitation_twice_returns_conflict() {
    let server = create_auth_test_server().await;
    let (_owner_id, owner_cookie) = register_user(&server, "owner@test.com", "Owner").await;
    let (_invitee_id, invitee_cookie) = register_user(&server, "invitee@test.com", "Invitee").await;

    let resp = server
        .post("/api/projects")
        .add_header(header::COOKIE, &owner_cookie)
        .json(&serde_json::json!({ "name": "Test Project" }))
        .await;
    let project_id = resp.json::<serde_json::Value>()["id"]
        .as_str()
        .unwrap()
        .to_string();

    let resp = server
        .post(&format!("/api/projects/{}/invitations", project_id))
        .add_header(header::COOKIE, &owner_cookie)
        .json(&serde_json::json!({}))
        .await;
    let token = resp.json::<serde_json::Value>()["token"]
        .as_str()
        .unwrap()
        .to_string();

    // Accept first time
    server
        .post(&format!("/api/invitations/{}/accept", token))
        .add_header(header::COOKIE, &invitee_cookie)
        .await
        .assert_status_ok();

    // Accept second time — should be conflict
    let resp = server
        .post(&format!("/api/invitations/{}/accept", token))
        .add_header(header::COOKIE, &invitee_cookie)
        .await;

    resp.assert_status(StatusCode::CONFLICT);
}
