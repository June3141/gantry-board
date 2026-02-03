use sqlx::SqlitePool;
use uuid::Uuid;

use crate::error::{AppError, AppResult};
use crate::models::project::{AddMemberRequest, MemberRole, ProjectMember, UpdateMemberRequest};

pub async fn add_member(
    _pool: &SqlitePool,
    _project_id: Uuid,
    _req: &AddMemberRequest,
) -> AppResult<ProjectMember> {
    Err(AppError::Internal(anyhow::anyhow!("not implemented")))
}

pub async fn get_member(
    _pool: &SqlitePool,
    _project_id: Uuid,
    _user_id: Uuid,
) -> AppResult<ProjectMember> {
    Err(AppError::Internal(anyhow::anyhow!("not implemented")))
}

pub async fn list_members(_pool: &SqlitePool, _project_id: Uuid) -> AppResult<Vec<ProjectMember>> {
    Err(AppError::Internal(anyhow::anyhow!("not implemented")))
}

pub async fn update_member_role(
    _pool: &SqlitePool,
    _project_id: Uuid,
    _user_id: Uuid,
    _req: &UpdateMemberRequest,
) -> AppResult<ProjectMember> {
    Err(AppError::Internal(anyhow::anyhow!("not implemented")))
}

pub async fn remove_member(_pool: &SqlitePool, _project_id: Uuid, _user_id: Uuid) -> AppResult<()> {
    Err(AppError::Internal(anyhow::anyhow!("not implemented")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::project_service;
    use crate::models::project::CreateProjectRequest;
    use sqlx::sqlite::SqlitePoolOptions;

    async fn setup_test_db() -> SqlitePool {
        let pool = SqlitePoolOptions::new()
            .connect("sqlite::memory:")
            .await
            .expect("Failed to create test database");

        sqlx::migrate!("./migrations")
            .run(&pool)
            .await
            .expect("Failed to run migrations");

        pool
    }

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

    #[tokio::test]
    async fn test_add_member_creates_membership() {
        let pool = setup_test_db().await;
        let project_id = create_test_project(&pool).await;
        let user_id = Uuid::new_v4();

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
    }

    #[tokio::test]
    async fn test_add_member_with_owner_role() {
        let pool = setup_test_db().await;
        let project_id = create_test_project(&pool).await;
        let user_id = Uuid::new_v4();

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
        let user_id = Uuid::new_v4();

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

        let user1 = Uuid::new_v4();
        let user2 = Uuid::new_v4();

        add_member(&pool, project_id, &AddMemberRequest {
            user_id: user1,
            role: MemberRole::Owner,
        }).await.expect("Failed to add member 1");

        add_member(&pool, project_id, &AddMemberRequest {
            user_id: user2,
            role: MemberRole::Member,
        }).await.expect("Failed to add member 2");

        let members = list_members(&pool, project_id)
            .await
            .expect("Failed to list members");

        assert_eq!(members.len(), 2);
    }

    #[tokio::test]
    async fn test_update_member_role_changes_role() {
        let pool = setup_test_db().await;
        let project_id = create_test_project(&pool).await;
        let user_id = Uuid::new_v4();

        add_member(&pool, project_id, &AddMemberRequest {
            user_id,
            role: MemberRole::Member,
        }).await.expect("Failed to add member");

        let updated = update_member_role(&pool, project_id, user_id, &UpdateMemberRequest {
            role: MemberRole::Admin,
        }).await.expect("Failed to update role");

        assert!(matches!(updated.role, MemberRole::Admin));
    }

    #[tokio::test]
    async fn test_remove_member_deletes_membership() {
        let pool = setup_test_db().await;
        let project_id = create_test_project(&pool).await;
        let user_id = Uuid::new_v4();

        add_member(&pool, project_id, &AddMemberRequest {
            user_id,
            role: MemberRole::Member,
        }).await.expect("Failed to add member");

        remove_member(&pool, project_id, user_id)
            .await
            .expect("Failed to remove member");

        let result = get_member(&pool, project_id, user_id).await;
        assert!(matches!(result, Err(AppError::NotFound(_))));
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
