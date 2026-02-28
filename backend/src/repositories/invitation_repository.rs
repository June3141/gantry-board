use chrono::{DateTime, Utc};
use sqlx::prelude::FromRow;
use sqlx::sqlite::SqliteConnection;
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::error::{AppError, AppResult};
use crate::models::project_invitation::ProjectInvitation;

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

fn row_to_invitation(row: InvitationRow) -> AppResult<ProjectInvitation> {
    row.try_into()
}

pub async fn insert(
    pool: &SqlitePool,
    id: Uuid,
    project_id: Uuid,
    invited_by: Uuid,
    token_hash: &str,
    role: &str,
    expires_at: DateTime<Utc>,
) -> AppResult<()> {
    sqlx::query(
        r#"
        INSERT INTO project_invitations (id, project_id, invited_by, token_hash, role, expires_at)
        VALUES ($1, $2, $3, $4, $5, $6)
        "#,
    )
    .bind(id.to_string())
    .bind(project_id.to_string())
    .bind(invited_by.to_string())
    .bind(token_hash)
    .bind(role)
    .bind(expires_at)
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn find_by_id(pool: &SqlitePool, invitation_id: Uuid) -> AppResult<ProjectInvitation> {
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
        .and_then(row_to_invitation)
}

pub async fn find_all_by_project(
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

    rows.into_iter().map(row_to_invitation).collect()
}

pub async fn delete(pool: &SqlitePool, invitation_id: Uuid, project_id: Uuid) -> AppResult<u64> {
    let result = sqlx::query("DELETE FROM project_invitations WHERE id = $1 AND project_id = $2")
        .bind(invitation_id.to_string())
        .bind(project_id.to_string())
        .execute(pool)
        .await?;

    Ok(result.rows_affected())
}

pub async fn find_by_token_hash(
    pool: &SqlitePool,
    token_hash: &str,
) -> AppResult<Option<ProjectInvitation>> {
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
    .bind(token_hash)
    .fetch_optional(pool)
    .await?;

    row.map(row_to_invitation).transpose()
}

pub async fn mark_accepted_tx(
    conn: &mut SqliteConnection,
    invitation_id: Uuid,
    user_id: Uuid,
    now: DateTime<Utc>,
) -> AppResult<u64> {
    let result = sqlx::query(
        "UPDATE project_invitations SET accepted_at = $1, accepted_by = $2 WHERE id = $3 AND accepted_at IS NULL",
    )
    .bind(now)
    .bind(user_id.to_string())
    .bind(invitation_id.to_string())
    .execute(&mut *conn)
    .await?;

    Ok(result.rows_affected())
}

pub async fn is_project_member_tx(
    conn: &mut SqliteConnection,
    project_id: Uuid,
    user_id: Uuid,
) -> AppResult<bool> {
    let count = sqlx::query_scalar::<_, i32>(
        "SELECT COUNT(*) FROM project_members WHERE project_id = $1 AND user_id = $2",
    )
    .bind(project_id.to_string())
    .bind(user_id.to_string())
    .fetch_one(&mut *conn)
    .await?;

    Ok(count > 0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::project::CreateProjectRequest;
    use crate::models::user::RegisterRequest;
    use crate::services::{project_service, user_service};
    use crate::test_helpers::setup_test_db;

    async fn create_test_project(pool: &SqlitePool) -> Uuid {
        project_service::create_project(
            pool,
            &CreateProjectRequest {
                name: "Test Project".to_string(),
                description: None,
                repository_path: None,
            },
        )
        .await
        .expect("create project")
        .id
    }

    async fn create_test_user(pool: &SqlitePool, email: &str, name: &str) -> Uuid {
        let req = RegisterRequest {
            email: email.to_string(),
            name: name.to_string(),
            password: "correct horse battery staple purple".to_string(),
        };
        user_service::create_user(pool, &req)
            .await
            .expect("create user")
            .id
    }

    async fn create_test_invitation(
        pool: &SqlitePool,
        project_id: Uuid,
        invited_by: Uuid,
        token_hash: &str,
    ) -> Uuid {
        let id = Uuid::new_v4();
        let expires_at = Utc::now() + chrono::Duration::hours(72);
        insert(
            pool, id, project_id, invited_by, token_hash, "member", expires_at,
        )
        .await
        .expect("insert invitation");
        id
    }

    #[tokio::test]
    async fn test_insert_and_find_by_id() {
        let pool = setup_test_db().await;
        let project_id = create_test_project(&pool).await;
        let user_id = create_test_user(&pool, "inviter@test.com", "Inviter").await;

        let id = create_test_invitation(&pool, project_id, user_id, "hash_abc").await;

        let invitation = find_by_id(&pool, id).await.expect("find_by_id");
        assert_eq!(invitation.id, id);
        assert_eq!(invitation.project_id, project_id);
        assert_eq!(invitation.invited_by, user_id);
        assert_eq!(invitation.invited_by_name, "Inviter");
        assert_eq!(invitation.project_name, "Test Project");
        assert!(invitation.accepted_at.is_none());
    }

    #[tokio::test]
    async fn test_find_by_id_not_found() {
        let pool = setup_test_db().await;
        let result = find_by_id(&pool, Uuid::new_v4()).await;
        assert!(matches!(result, Err(AppError::NotFound(_))));
    }

    #[tokio::test]
    async fn test_find_all_by_project() {
        let pool = setup_test_db().await;
        let project_id = create_test_project(&pool).await;
        let user_id = create_test_user(&pool, "inviter@test.com", "Inviter").await;

        create_test_invitation(&pool, project_id, user_id, "hash_1").await;
        create_test_invitation(&pool, project_id, user_id, "hash_2").await;

        let invitations = find_all_by_project(&pool, project_id)
            .await
            .expect("find_all_by_project");
        assert_eq!(invitations.len(), 2);
    }

    #[tokio::test]
    async fn test_find_all_by_project_empty() {
        let pool = setup_test_db().await;
        let project_id = create_test_project(&pool).await;

        let invitations = find_all_by_project(&pool, project_id)
            .await
            .expect("find_all_by_project");
        assert!(invitations.is_empty());
    }

    #[tokio::test]
    async fn test_delete() {
        let pool = setup_test_db().await;
        let project_id = create_test_project(&pool).await;
        let user_id = create_test_user(&pool, "inviter@test.com", "Inviter").await;

        let id = create_test_invitation(&pool, project_id, user_id, "hash_del").await;

        let rows = delete(&pool, id, project_id).await.expect("delete");
        assert_eq!(rows, 1);

        let result = find_by_id(&pool, id).await;
        assert!(matches!(result, Err(AppError::NotFound(_))));
    }

    #[tokio::test]
    async fn test_delete_nonexistent() {
        let pool = setup_test_db().await;
        let project_id = create_test_project(&pool).await;

        let rows = delete(&pool, Uuid::new_v4(), project_id)
            .await
            .expect("delete");
        assert_eq!(rows, 0);
    }

    #[tokio::test]
    async fn test_find_by_token_hash_found() {
        let pool = setup_test_db().await;
        let project_id = create_test_project(&pool).await;
        let user_id = create_test_user(&pool, "inviter@test.com", "Inviter").await;

        let id = create_test_invitation(&pool, project_id, user_id, "unique_hash").await;

        let result = find_by_token_hash(&pool, "unique_hash")
            .await
            .expect("find_by_token_hash");
        assert!(result.is_some());
        assert_eq!(result.unwrap().id, id);
    }

    #[tokio::test]
    async fn test_find_by_token_hash_not_found() {
        let pool = setup_test_db().await;

        let result = find_by_token_hash(&pool, "nonexistent_hash")
            .await
            .expect("find_by_token_hash");
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_mark_accepted_tx() {
        let pool = setup_test_db().await;
        let project_id = create_test_project(&pool).await;
        let inviter_id = create_test_user(&pool, "inviter@test.com", "Inviter").await;
        let acceptor_id = create_test_user(&pool, "acceptor@test.com", "Acceptor").await;

        let id = create_test_invitation(&pool, project_id, inviter_id, "hash_accept").await;

        let now = Utc::now();
        let mut tx = pool.begin().await.unwrap();
        let rows = mark_accepted_tx(&mut *tx, id, acceptor_id, now)
            .await
            .expect("mark_accepted_tx");
        tx.commit().await.unwrap();

        assert_eq!(rows, 1);

        let invitation = find_by_id(&pool, id).await.expect("find_by_id");
        assert!(invitation.accepted_at.is_some());
        assert_eq!(invitation.accepted_by, Some(acceptor_id));
    }

    #[tokio::test]
    async fn test_mark_accepted_tx_already_accepted() {
        let pool = setup_test_db().await;
        let project_id = create_test_project(&pool).await;
        let inviter_id = create_test_user(&pool, "inviter@test.com", "Inviter").await;
        let acceptor_id = create_test_user(&pool, "acceptor@test.com", "Acceptor").await;

        let id = create_test_invitation(&pool, project_id, inviter_id, "hash_double").await;

        let now = Utc::now();
        let mut tx = pool.begin().await.unwrap();
        mark_accepted_tx(&mut *tx, id, acceptor_id, now)
            .await
            .expect("first accept");
        tx.commit().await.unwrap();

        // Second accept should return 0 rows
        let mut tx2 = pool.begin().await.unwrap();
        let rows = mark_accepted_tx(&mut *tx2, id, acceptor_id, now)
            .await
            .expect("second accept");
        tx2.commit().await.unwrap();

        assert_eq!(rows, 0);
    }

    #[tokio::test]
    async fn test_is_project_member_tx() {
        let pool = setup_test_db().await;
        let project_id = create_test_project(&pool).await;
        let user_id = create_test_user(&pool, "member@test.com", "Member").await;

        // Not a member yet
        let mut tx = pool.begin().await.unwrap();
        let is_member = is_project_member_tx(&mut *tx, project_id, user_id)
            .await
            .expect("is_project_member_tx");
        tx.commit().await.unwrap();
        assert!(!is_member);

        // Add as member
        use crate::models::project::AddMemberRequest;
        use crate::models::project::MemberRole;
        use crate::services::member_service;
        member_service::add_member(
            &pool,
            project_id,
            &AddMemberRequest {
                user_id,
                role: MemberRole::Member,
            },
        )
        .await
        .expect("add member");

        // Now is a member
        let mut tx2 = pool.begin().await.unwrap();
        let is_member = is_project_member_tx(&mut *tx2, project_id, user_id)
            .await
            .expect("is_project_member_tx");
        tx2.commit().await.unwrap();
        assert!(is_member);
    }
}
