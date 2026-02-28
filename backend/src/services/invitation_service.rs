use chrono::{Duration, Utc};
use rand::Rng;
use sha2::{Digest, Sha256};
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::error::{AppError, AppResult};
use crate::models::project::MemberRole;
use crate::models::project_invitation::{InvitationInfo, ProjectInvitation};
use crate::repositories::invitation_repository;

const TOKEN_BYTES: usize = 32; // 256-bit
const EXPIRY_HOURS: i64 = 72;

/// Generate a cryptographically random token (hex-encoded).
///
/// Uses `OsRng` directly for cryptographic randomness, avoiding the
/// reseeding overhead of `thread_rng()`.
pub fn generate_token() -> String {
    let mut bytes = [0u8; TOKEN_BYTES];
    rand::rngs::OsRng.fill(&mut bytes);
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// Hash a token with SHA-256 for DB storage.
///
/// Unsalted SHA-256 is safe here because the input token has 256 bits of
/// entropy (from `generate_token`), making brute-force and rainbow-table
/// attacks infeasible. HMAC would add no practical security benefit for
/// high-entropy random tokens.
pub fn hash_token(token: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(token.as_bytes());
    let result = hasher.finalize();
    result.iter().map(|b| format!("{b:02x}")).collect()
}

pub async fn create_invitation(
    pool: &SqlitePool,
    project_id: Uuid,
    invited_by: Uuid,
    role: MemberRole,
) -> AppResult<(ProjectInvitation, String)> {
    let id = Uuid::new_v4();
    let token = generate_token();
    let token_hash = hash_token(&token);
    let expires_at = Utc::now() + Duration::hours(EXPIRY_HOURS);

    let role_str = serde_json::to_value(&role)
        .map_err(|e| AppError::Internal(e.to_string()))?
        .as_str()
        .unwrap_or("member")
        .to_string();

    invitation_repository::insert(
        pool,
        id,
        project_id,
        invited_by,
        &token_hash,
        &role_str,
        expires_at,
    )
    .await?;

    let invitation = get_invitation(pool, id).await?;
    Ok((invitation, token))
}

pub async fn get_invitation(
    pool: &SqlitePool,
    invitation_id: Uuid,
) -> AppResult<ProjectInvitation> {
    invitation_repository::find_by_id(pool, invitation_id).await
}

pub async fn list_invitations(
    pool: &SqlitePool,
    project_id: Uuid,
) -> AppResult<Vec<ProjectInvitation>> {
    invitation_repository::find_all_by_project(pool, project_id).await
}

pub async fn delete_invitation(
    pool: &SqlitePool,
    project_id: Uuid,
    invitation_id: Uuid,
) -> AppResult<()> {
    let rows = invitation_repository::delete(pool, invitation_id, project_id).await?;
    if rows == 0 {
        return Err(AppError::NotFound(format!(
            "invitation not found: {invitation_id}"
        )));
    }
    Ok(())
}

pub async fn get_invitation_by_token(pool: &SqlitePool, token: &str) -> AppResult<InvitationInfo> {
    let token_hash = hash_token(token);

    let invitation = invitation_repository::find_by_token_hash(pool, &token_hash)
        .await?
        .ok_or_else(|| AppError::NotFound("invalid invitation token".to_string()))?;

    Ok(InvitationInfo {
        id: invitation.id,
        project_name: invitation.project_name,
        invited_by_name: invitation.invited_by_name,
        role: invitation.role,
        expires_at: invitation.expires_at,
        expired: invitation.expires_at < Utc::now(),
        accepted: invitation.accepted_at.is_some(),
    })
}

pub async fn accept_invitation(
    pool: &SqlitePool,
    token: &str,
    user_id: Uuid,
) -> AppResult<ProjectInvitation> {
    let token_hash = hash_token(token);

    let invitation = invitation_repository::find_by_token_hash(pool, &token_hash)
        .await?
        .ok_or_else(|| AppError::NotFound("invalid invitation token".to_string()))?;

    if invitation.accepted_at.is_some() {
        return Err(AppError::Conflict(
            "invitation has already been accepted".to_string(),
        ));
    }

    if invitation.expires_at < Utc::now() {
        return Err(AppError::Validation("invitation has expired".to_string()));
    }

    let mut tx = pool.begin().await?;

    // Mark invitation as accepted (conditional on accepted_at still being NULL
    // to prevent double-accept race condition)
    let rows =
        invitation_repository::mark_accepted_tx(&mut *tx, invitation.id, user_id, Utc::now())
            .await?;

    if rows == 0 {
        return Err(AppError::Conflict(
            "invitation has already been accepted".to_string(),
        ));
    }

    // Add user as project member
    use crate::models::project::AddMemberRequest;
    use crate::services::member_service;

    // Check if already a member
    let is_member =
        invitation_repository::is_project_member_tx(&mut *tx, invitation.project_id, user_id)
            .await?;

    if !is_member {
        member_service::add_member_tx(
            &mut tx,
            invitation.project_id,
            &AddMemberRequest {
                user_id,
                role: invitation.role.clone(),
            },
        )
        .await?;
    }

    tx.commit().await?;

    get_invitation(pool, invitation.id).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::project::CreateProjectRequest;
    use crate::models::user::RegisterRequest;
    use crate::services::{project_service, user_service};
    use crate::test_helpers::setup_test_db;

    async fn setup_project_and_user(pool: &SqlitePool) -> (Uuid, Uuid) {
        let user = user_service::create_user(
            pool,
            &RegisterRequest {
                email: "inviter@test.com".to_string(),
                name: "Inviter".to_string(),
                password: "password123".to_string(),
            },
        )
        .await
        .unwrap();

        let project = project_service::create_project(
            pool,
            &CreateProjectRequest {
                name: "Test Project".to_string(),
                description: None,
                repository_path: None,
            },
        )
        .await
        .unwrap();

        (project.id, user.id)
    }

    #[test]
    fn test_generate_token_returns_64_hex_chars() {
        let token = generate_token();
        assert_eq!(token.len(), 64); // 32 bytes * 2 hex chars
        assert!(token.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn test_generate_token_is_unique() {
        let t1 = generate_token();
        let t2 = generate_token();
        assert_ne!(t1, t2);
    }

    #[test]
    fn test_hash_token_is_deterministic() {
        let token = "abc123";
        assert_eq!(hash_token(token), hash_token(token));
    }

    #[test]
    fn test_hash_token_differs_for_different_input() {
        assert_ne!(hash_token("token_a"), hash_token("token_b"));
    }

    #[tokio::test]
    async fn test_create_invitation_succeeds() {
        let pool = setup_test_db().await;
        let (project_id, user_id) = setup_project_and_user(&pool).await;

        let (invitation, token) = create_invitation(&pool, project_id, user_id, MemberRole::Member)
            .await
            .unwrap();

        assert_eq!(invitation.project_id, project_id);
        assert_eq!(invitation.invited_by, user_id);
        assert!(!token.is_empty());
        assert!(invitation.expires_at > Utc::now());
        assert!(invitation.accepted_at.is_none());
    }

    #[tokio::test]
    async fn test_get_invitation_by_token_returns_info() {
        let pool = setup_test_db().await;
        let (project_id, user_id) = setup_project_and_user(&pool).await;

        let (_, token) = create_invitation(&pool, project_id, user_id, MemberRole::Member)
            .await
            .unwrap();

        let info = get_invitation_by_token(&pool, &token).await.unwrap();
        assert_eq!(info.project_name, "Test Project");
        assert!(!info.expired);
        assert!(!info.accepted);
    }

    #[tokio::test]
    async fn test_get_invitation_by_invalid_token_fails() {
        let pool = setup_test_db().await;

        let result = get_invitation_by_token(&pool, "nonexistent_token").await;
        assert!(matches!(result, Err(AppError::NotFound(_))));
    }

    #[tokio::test]
    async fn test_list_invitations_returns_all() {
        let pool = setup_test_db().await;
        let (project_id, user_id) = setup_project_and_user(&pool).await;

        create_invitation(&pool, project_id, user_id, MemberRole::Member)
            .await
            .unwrap();
        create_invitation(&pool, project_id, user_id, MemberRole::Admin)
            .await
            .unwrap();

        let list = list_invitations(&pool, project_id).await.unwrap();
        assert_eq!(list.len(), 2);
    }

    #[tokio::test]
    async fn test_delete_invitation_succeeds() {
        let pool = setup_test_db().await;
        let (project_id, user_id) = setup_project_and_user(&pool).await;

        let (invitation, _) = create_invitation(&pool, project_id, user_id, MemberRole::Member)
            .await
            .unwrap();

        delete_invitation(&pool, project_id, invitation.id)
            .await
            .unwrap();

        let list = list_invitations(&pool, project_id).await.unwrap();
        assert!(list.is_empty());
    }

    #[tokio::test]
    async fn test_accept_invitation_adds_member() {
        let pool = setup_test_db().await;
        let (project_id, user_id) = setup_project_and_user(&pool).await;

        let acceptor = user_service::create_user(
            &pool,
            &RegisterRequest {
                email: "acceptor@test.com".to_string(),
                name: "Acceptor".to_string(),
                password: "password123".to_string(),
            },
        )
        .await
        .unwrap();

        let (_, token) = create_invitation(&pool, project_id, user_id, MemberRole::Member)
            .await
            .unwrap();

        let accepted = accept_invitation(&pool, &token, acceptor.id).await.unwrap();
        assert!(accepted.accepted_at.is_some());
        assert_eq!(accepted.accepted_by, Some(acceptor.id));
    }

    #[tokio::test]
    async fn test_accept_invitation_twice_fails() {
        let pool = setup_test_db().await;
        let (project_id, user_id) = setup_project_and_user(&pool).await;

        let acceptor = user_service::create_user(
            &pool,
            &RegisterRequest {
                email: "acceptor2@test.com".to_string(),
                name: "Acceptor2".to_string(),
                password: "password123".to_string(),
            },
        )
        .await
        .unwrap();

        let (_, token) = create_invitation(&pool, project_id, user_id, MemberRole::Member)
            .await
            .unwrap();

        accept_invitation(&pool, &token, acceptor.id).await.unwrap();
        let result = accept_invitation(&pool, &token, acceptor.id).await;
        assert!(matches!(result, Err(AppError::Conflict(_))));
    }
}
