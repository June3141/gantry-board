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
pub fn generate_token() -> String {
    let mut bytes = [0u8; TOKEN_BYTES];
    rand::thread_rng().fill(&mut bytes);
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// Hash a token with SHA-256 for DB storage.
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
