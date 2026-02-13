use chrono::{DateTime, Utc};
use sqlx::prelude::FromRow;
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::error::{AppError, AppResult};
use crate::models::project::{AddMemberRequest, MemberRole, ProjectMember, UpdateMemberRequest};
use crate::services::user_service;

#[derive(FromRow)]
struct MemberRow {
    project_id: String,
    user_id: String,
    role: MemberRole,
    user_name: String,
    user_email: String,
    created_at: DateTime<Utc>,
}

impl TryFrom<MemberRow> for ProjectMember {
    type Error = uuid::Error;

    fn try_from(row: MemberRow) -> Result<Self, Self::Error> {
        Ok(ProjectMember {
            project_id: row.project_id.parse()?,
            user_id: row.user_id.parse()?,
            role: row.role,
            user_name: row.user_name,
            user_email: row.user_email,
            created_at: row.created_at,
        })
    }
}

#[tracing::instrument(skip(pool, req), fields(%project_id, user_id = %req.user_id))]
pub async fn add_member(
    pool: &SqlitePool,
    project_id: Uuid,
    req: &AddMemberRequest,
) -> AppResult<ProjectMember> {
    // Validate that the user exists before creating the membership
    user_service::get_user(pool, req.user_id).await?;

    let now = Utc::now();

    sqlx::query(
        r#"
        INSERT INTO project_members (project_id, user_id, role, created_at)
        VALUES ($1, $2, $3, $4)
        "#,
    )
    .bind(project_id.to_string())
    .bind(req.user_id.to_string())
    .bind(&req.role)
    .bind(now)
    .execute(pool)
    .await?;

    get_member(pool, project_id, req.user_id).await
}

pub async fn get_member(
    pool: &SqlitePool,
    project_id: Uuid,
    user_id: Uuid,
) -> AppResult<ProjectMember> {
    let row = sqlx::query_as::<_, MemberRow>(
        r#"
        SELECT pm.project_id, pm.user_id, pm.role,
               u.name as user_name, u.email as user_email,
               pm.created_at
        FROM project_members pm
        JOIN users u ON pm.user_id = u.id
        WHERE pm.project_id = $1 AND pm.user_id = $2
        "#,
    )
    .bind(project_id.to_string())
    .bind(user_id.to_string())
    .fetch_optional(pool)
    .await?;

    row.map(|r| r.try_into())
        .transpose()
        .map_err(|e: uuid::Error| AppError::Internal(e.to_string()))?
        .ok_or_else(|| {
            AppError::NotFound(format!(
                "member {} not found in project {}",
                user_id, project_id
            ))
        })
}

pub async fn list_members(pool: &SqlitePool, project_id: Uuid) -> AppResult<Vec<ProjectMember>> {
    let rows = sqlx::query_as::<_, MemberRow>(
        r#"
        SELECT pm.project_id, pm.user_id, pm.role,
               u.name as user_name, u.email as user_email,
               pm.created_at
        FROM project_members pm
        JOIN users u ON pm.user_id = u.id
        WHERE pm.project_id = $1
        ORDER BY pm.created_at ASC
        "#,
    )
    .bind(project_id.to_string())
    .fetch_all(pool)
    .await?;

    rows.into_iter()
        .map(|r| r.try_into())
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e: uuid::Error| AppError::Internal(e.to_string()))
}

#[tracing::instrument(skip(pool, req), fields(%project_id, %user_id))]
pub async fn update_member_role(
    pool: &SqlitePool,
    project_id: Uuid,
    user_id: Uuid,
    req: &UpdateMemberRequest,
) -> AppResult<ProjectMember> {
    // Verify member exists
    get_member(pool, project_id, user_id).await?;

    sqlx::query(
        r#"
        UPDATE project_members
        SET role = $1
        WHERE project_id = $2 AND user_id = $3
        "#,
    )
    .bind(&req.role)
    .bind(project_id.to_string())
    .bind(user_id.to_string())
    .execute(pool)
    .await?;

    get_member(pool, project_id, user_id).await
}

#[tracing::instrument(skip(pool), fields(%project_id, %user_id))]
pub async fn remove_member(pool: &SqlitePool, project_id: Uuid, user_id: Uuid) -> AppResult<()> {
    let result = sqlx::query(
        r#"
        DELETE FROM project_members
        WHERE project_id = $1 AND user_id = $2
        "#,
    )
    .bind(project_id.to_string())
    .bind(user_id.to_string())
    .execute(pool)
    .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound(format!(
            "member {} not found in project {}",
            user_id, project_id
        )));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::project::CreateProjectRequest;
    use crate::models::user::RegisterRequest;
    use crate::services::{project_service, user_service};
    use crate::test_helpers::setup_test_db;

    async fn create_test_project(pool: &SqlitePool) -> Uuid {
        let req = CreateProjectRequest {
            name: "Test Project".to_string(),
            description: None,
        };
        let project = project_service::create_project(pool, &req)
            .await
            .expect("Failed to create project");
        project.id
    }

    async fn create_test_user(pool: &SqlitePool, email: &str, name: &str) -> Uuid {
        let req = RegisterRequest {
            email: email.to_string(),
            name: name.to_string(),
            password: "password123".to_string(),
        };
        user_service::create_user(pool, &req)
            .await
            .expect("Failed to create user")
            .id
    }

    #[tokio::test]
    async fn test_add_member_creates_membership() {
        let pool = setup_test_db().await;
        let project_id = create_test_project(&pool).await;
        let user_id = create_test_user(&pool, "alice@test.com", "Alice").await;

        let req = AddMemberRequest {
            user_id,
            role: MemberRole::Member,
        };
        let member = add_member(&pool, project_id, &req)
            .await
            .expect("Failed to add member");

        assert_eq!(member.project_id, project_id);
        assert_eq!(member.user_id, user_id);
        assert!(matches!(member.role, MemberRole::Member));
        assert_eq!(member.user_name, "Alice");
        assert_eq!(member.user_email, "alice@test.com");
    }

    #[tokio::test]
    async fn test_add_member_with_owner_role() {
        let pool = setup_test_db().await;
        let project_id = create_test_project(&pool).await;
        let user_id = create_test_user(&pool, "bob@test.com", "Bob").await;

        let req = AddMemberRequest {
            user_id,
            role: MemberRole::Owner,
        };
        let member = add_member(&pool, project_id, &req)
            .await
            .expect("Failed to add member");

        assert!(matches!(member.role, MemberRole::Owner));
    }

    #[tokio::test]
    async fn test_get_member_returns_existing() {
        let pool = setup_test_db().await;
        let project_id = create_test_project(&pool).await;
        let user_id = create_test_user(&pool, "alice@test.com", "Alice").await;

        let req = AddMemberRequest {
            user_id,
            role: MemberRole::Admin,
        };
        add_member(&pool, project_id, &req)
            .await
            .expect("Failed to add member");

        let member = get_member(&pool, project_id, user_id)
            .await
            .expect("Failed to get member");

        assert_eq!(member.user_id, user_id);
        assert!(matches!(member.role, MemberRole::Admin));
    }

    #[tokio::test]
    async fn test_get_member_returns_not_found() {
        let pool = setup_test_db().await;
        let project_id = create_test_project(&pool).await;
        let random_user_id = Uuid::new_v4();

        let result = get_member(&pool, project_id, random_user_id).await;

        assert!(matches!(result, Err(AppError::NotFound(_))));
    }

    #[tokio::test]
    async fn test_list_members_returns_empty_initially() {
        let pool = setup_test_db().await;
        let project_id = create_test_project(&pool).await;

        let members = list_members(&pool, project_id)
            .await
            .expect("Failed to list members");

        assert!(members.is_empty());
    }

    #[tokio::test]
    async fn test_list_members_returns_all_members() {
        let pool = setup_test_db().await;
        let project_id = create_test_project(&pool).await;

        let user1 = create_test_user(&pool, "alice@test.com", "Alice").await;
        let user2 = create_test_user(&pool, "bob@test.com", "Bob").await;

        add_member(
            &pool,
            project_id,
            &AddMemberRequest {
                user_id: user1,
                role: MemberRole::Owner,
            },
        )
        .await
        .expect("Failed to add member 1");

        add_member(
            &pool,
            project_id,
            &AddMemberRequest {
                user_id: user2,
                role: MemberRole::Member,
            },
        )
        .await
        .expect("Failed to add member 2");

        let members = list_members(&pool, project_id)
            .await
            .expect("Failed to list members");

        assert_eq!(members.len(), 2);
    }

    #[tokio::test]
    async fn test_update_member_role_changes_role() {
        let pool = setup_test_db().await;
        let project_id = create_test_project(&pool).await;
        let user_id = create_test_user(&pool, "alice@test.com", "Alice").await;

        add_member(
            &pool,
            project_id,
            &AddMemberRequest {
                user_id,
                role: MemberRole::Member,
            },
        )
        .await
        .expect("Failed to add member");

        let updated = update_member_role(
            &pool,
            project_id,
            user_id,
            &UpdateMemberRequest {
                role: MemberRole::Admin,
            },
        )
        .await
        .expect("Failed to update role");

        assert!(matches!(updated.role, MemberRole::Admin));
    }

    #[tokio::test]
    async fn test_remove_member_deletes_membership() {
        let pool = setup_test_db().await;
        let project_id = create_test_project(&pool).await;
        let user_id = create_test_user(&pool, "alice@test.com", "Alice").await;

        add_member(
            &pool,
            project_id,
            &AddMemberRequest {
                user_id,
                role: MemberRole::Member,
            },
        )
        .await
        .expect("Failed to add member");

        remove_member(&pool, project_id, user_id)
            .await
            .expect("Failed to remove member");

        let result = get_member(&pool, project_id, user_id).await;
        assert!(matches!(result, Err(AppError::NotFound(_))));
    }

    #[tokio::test]
    async fn test_add_member_nonexistent_user_leaves_no_row() {
        let pool = setup_test_db().await;
        let project_id = create_test_project(&pool).await;
        let nonexistent_user = Uuid::new_v4();

        // Attempt to add a nonexistent user as a member — should fail
        let result = add_member(
            &pool,
            project_id,
            &AddMemberRequest {
                user_id: nonexistent_user,
                role: MemberRole::Member,
            },
        )
        .await;

        assert!(result.is_err());

        // Verify no member row was created (atomicity guarantee)
        let members = list_members(&pool, project_id)
            .await
            .expect("list should succeed");
        assert!(
            members.is_empty(),
            "no member row should exist after failed add_member"
        );
    }

    #[tokio::test]
    async fn test_remove_nonexistent_member_returns_not_found() {
        let pool = setup_test_db().await;
        let project_id = create_test_project(&pool).await;
        let random_user_id = Uuid::new_v4();

        let result = remove_member(&pool, project_id, random_user_id).await;

        assert!(matches!(result, Err(AppError::NotFound(_))));
    }
}
