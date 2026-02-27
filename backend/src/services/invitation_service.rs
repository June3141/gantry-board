use chrono::{DateTime, Duration, Utc};
use rand::Rng;
use sha2::{Digest, Sha256};
use sqlx::{FromRow, SqlitePool};
use uuid::Uuid;

use crate::error::{AppError, AppResult};
use crate::models::project::MemberRole;
use crate::models::project_invitation::{InvitationInfo, ProjectInvitation};

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

#[derive(FromRow)]
struct InvitationRow {
    id: String,
    project_id: String,
    invited_by: String,
    invited_by_name: String,
    project_name: String,
    role: String,
    expires_at: DateTime<Utc>,
    accepted_at: Option<DateTime<Utc>>,
    accepted_by: Option<String>,
    created_at: DateTime<Utc>,
}

impl TryFrom<InvitationRow> for ProjectInvitation {
    type Error = AppError;

    fn try_from(row: InvitationRow) -> Result<Self, Self::Error> {
        Ok(Self {
            id: row
                .id
                .parse()
                .map_err(|e: uuid::Error| AppError::Internal(e.to_string()))?,
            project_id: row
                .project_id
                .parse()
                .map_err(|e: uuid::Error| AppError::Internal(e.to_string()))?,
            invited_by: row
                .invited_by
                .parse()
                .map_err(|e: uuid::Error| AppError::Internal(e.to_string()))?,
            invited_by_name: row.invited_by_name,
            project_name: row.project_name,
            role: serde_json::from_value(serde_json::Value::String(row.role))
                .map_err(|e| AppError::Internal(e.to_string()))?,
            expires_at: row.expires_at,
            accepted_at: row.accepted_at,
            accepted_by: row
                .accepted_by
                .map(|s| {
                    s.parse()
                        .map_err(|e: uuid::Error| AppError::Internal(e.to_string()))
                })
                .transpose()?,
            created_at: row.created_at,
        })
    }
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

    sqlx::query(
        r#"
        INSERT INTO project_invitations (id, project_id, invited_by, token_hash, role, expires_at)
        VALUES ($1, $2, $3, $4, $5, $6)
        "#,
    )
    .bind(id.to_string())
    .bind(project_id.to_string())
    .bind(invited_by.to_string())
    .bind(&token_hash)
    .bind(&role_str)
    .bind(expires_at)
    .execute(pool)
    .await?;

    let invitation = get_invitation(pool, id).await?;
    Ok((invitation, token))
}

pub async fn get_invitation(
    pool: &SqlitePool,
    invitation_id: Uuid,
) -> AppResult<ProjectInvitation> {
    let row = sqlx::query_as::<_, InvitationRow>(
        r#"
        SELECT i.id, i.project_id, i.invited_by, u.name AS invited_by_name,
               p.name AS project_name, i.role, i.expires_at, i.accepted_at,
               i.accepted_by, i.created_at
        FROM project_invitations i
        JOIN users u ON i.invited_by = u.id
        JOIN projects p ON i.project_id = p.id
        WHERE i.id = $1
        "#,
    )
    .bind(invitation_id.to_string())
    .fetch_optional(pool)
    .await?;

    row.ok_or_else(|| AppError::NotFound(format!("invitation not found: {invitation_id}")))
        .and_then(ProjectInvitation::try_from)
}

pub async fn list_invitations(
    pool: &SqlitePool,
    project_id: Uuid,
) -> AppResult<Vec<ProjectInvitation>> {
    let rows = sqlx::query_as::<_, InvitationRow>(
        r#"
        SELECT i.id, i.project_id, i.invited_by, u.name AS invited_by_name,
               p.name AS project_name, i.role, i.expires_at, i.accepted_at,
               i.accepted_by, i.created_at
        FROM project_invitations i
        JOIN users u ON i.invited_by = u.id
        JOIN projects p ON i.project_id = p.id
        WHERE i.project_id = $1
        ORDER BY i.created_at DESC
        "#,
    )
    .bind(project_id.to_string())
    .fetch_all(pool)
    .await?;

    rows.into_iter().map(ProjectInvitation::try_from).collect()
}

pub async fn delete_invitation(
    pool: &SqlitePool,
    project_id: Uuid,
    invitation_id: Uuid,
) -> AppResult<()> {
    let result = sqlx::query("DELETE FROM project_invitations WHERE id = $1 AND project_id = $2")
        .bind(invitation_id.to_string())
        .bind(project_id.to_string())
        .execute(pool)
        .await?;
    if result.rows_affected() == 0 {
        return Err(AppError::NotFound(format!(
            "invitation not found: {invitation_id}"
        )));
    }
    Ok(())
}

pub async fn get_invitation_by_token(pool: &SqlitePool, token: &str) -> AppResult<InvitationInfo> {
    let token_hash = hash_token(token);

    let row = sqlx::query_as::<_, InvitationRow>(
        r#"
        SELECT i.id, i.project_id, i.invited_by, u.name AS invited_by_name,
               p.name AS project_name, i.role, i.expires_at, i.accepted_at,
               i.accepted_by, i.created_at
        FROM project_invitations i
        JOIN users u ON i.invited_by = u.id
        JOIN projects p ON i.project_id = p.id
        WHERE i.token_hash = $1
        "#,
    )
    .bind(&token_hash)
    .fetch_optional(pool)
    .await?;

    let invitation = row
        .ok_or_else(|| AppError::NotFound("invalid invitation token".to_string()))
        .and_then(ProjectInvitation::try_from)?;

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

    let row = sqlx::query_as::<_, InvitationRow>(
        r#"
        SELECT i.id, i.project_id, i.invited_by, u.name AS invited_by_name,
               p.name AS project_name, i.role, i.expires_at, i.accepted_at,
               i.accepted_by, i.created_at
        FROM project_invitations i
        JOIN users u ON i.invited_by = u.id
        JOIN projects p ON i.project_id = p.id
        WHERE i.token_hash = $1
        "#,
    )
    .bind(&token_hash)
    .fetch_optional(pool)
    .await?;

    let invitation = row
        .ok_or_else(|| AppError::NotFound("invalid invitation token".to_string()))
        .and_then(ProjectInvitation::try_from)?;

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
    let result = sqlx::query(
        "UPDATE project_invitations SET accepted_at = $1, accepted_by = $2 WHERE id = $3 AND accepted_at IS NULL",
    )
    .bind(Utc::now())
    .bind(user_id.to_string())
    .bind(invitation.id.to_string())
    .execute(&mut *tx)
    .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::Conflict(
            "invitation has already been accepted".to_string(),
        ));
    }

    // Add user as project member
    use crate::models::project::AddMemberRequest;
    use crate::services::member_service;

    // Check if already a member
    let existing = sqlx::query_scalar::<_, i32>(
        "SELECT COUNT(*) FROM project_members WHERE project_id = $1 AND user_id = $2",
    )
    .bind(invitation.project_id.to_string())
    .bind(user_id.to_string())
    .fetch_one(&mut *tx)
    .await?;

    if existing == 0 {
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
