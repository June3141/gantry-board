use sqlx::SqlitePool;
use uuid::Uuid;

use crate::error::AppResult;
use crate::models::project::MemberRole;

/// Check if user is a member of the project (any role).
/// Returns the member's role or Forbidden.
/// When user_id is nil (auth_disabled mode), returns Owner to grant full access.
pub async fn require_project_member(
    pool: &SqlitePool,
    user_id: Uuid,
    project_id: Uuid,
) -> AppResult<MemberRole> {
    todo!()
}

/// Check if user has Owner or Admin role in the project.
/// Returns the member's role or Forbidden.
pub async fn require_project_admin(
    pool: &SqlitePool,
    user_id: Uuid,
    project_id: Uuid,
) -> AppResult<MemberRole> {
    todo!()
}

/// Check if user is Owner of the project.
/// Returns Ok(()) or Forbidden.
pub async fn require_project_owner(
    pool: &SqlitePool,
    user_id: Uuid,
    project_id: Uuid,
) -> AppResult<()> {
    todo!()
}

/// List project IDs the user is a member of.
pub async fn list_user_project_ids(pool: &SqlitePool, user_id: Uuid) -> AppResult<Vec<Uuid>> {
    todo!()
}

/// Count owners in a project.
pub async fn count_owners(pool: &SqlitePool, project_id: Uuid) -> AppResult<i64> {
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::AppError;
    use crate::models::project::AddMemberRequest;
    use crate::services::{member_service, project_service};
    use crate::test_helpers::setup_test_db;

    use crate::models::project::CreateProjectRequest;

    async fn setup_project_with_member(pool: &SqlitePool, role: MemberRole) -> (Uuid, Uuid) {
        let project = project_service::create_project(
            pool,
            &CreateProjectRequest {
                name: "Test Project".to_string(),
                description: None,
            },
        )
        .await
        .unwrap();
        let user_id = Uuid::new_v4();
        member_service::add_member(pool, project.id, &AddMemberRequest { user_id, role })
            .await
            .unwrap();
        (project.id, user_id)
    }

    // --- require_project_member ---

    #[tokio::test]
    async fn test_require_project_member_returns_role_for_owner() {
        let pool = setup_test_db().await;
        let (project_id, user_id) = setup_project_with_member(&pool, MemberRole::Owner).await;

        let result = require_project_member(&pool, user_id, project_id).await;
        assert!(matches!(result, Ok(MemberRole::Owner)));
    }

    #[tokio::test]
    async fn test_require_project_member_returns_role_for_admin() {
        let pool = setup_test_db().await;
        let (project_id, user_id) = setup_project_with_member(&pool, MemberRole::Admin).await;

        let result = require_project_member(&pool, user_id, project_id).await;
        assert!(matches!(result, Ok(MemberRole::Admin)));
    }

    #[tokio::test]
    async fn test_require_project_member_returns_role_for_member() {
        let pool = setup_test_db().await;
        let (project_id, user_id) = setup_project_with_member(&pool, MemberRole::Member).await;

        let result = require_project_member(&pool, user_id, project_id).await;
        assert!(matches!(result, Ok(MemberRole::Member)));
    }

    #[tokio::test]
    async fn test_require_project_member_returns_forbidden_for_non_member() {
        let pool = setup_test_db().await;
        let project = project_service::create_project(
            &pool,
            &CreateProjectRequest {
                name: "Test".to_string(),
                description: None,
            },
        )
        .await
        .unwrap();
        let non_member = Uuid::new_v4();

        let result = require_project_member(&pool, non_member, project.id).await;
        assert!(matches!(result, Err(AppError::Forbidden(_))));
    }

    #[tokio::test]
    async fn test_require_project_member_skips_for_nil_user() {
        let pool = setup_test_db().await;
        let project = project_service::create_project(
            &pool,
            &CreateProjectRequest {
                name: "Test".to_string(),
                description: None,
            },
        )
        .await
        .unwrap();

        let result = require_project_member(&pool, Uuid::nil(), project.id).await;
        assert!(matches!(result, Ok(MemberRole::Owner)));
    }

    // --- require_project_admin ---

    #[tokio::test]
    async fn test_require_project_admin_allows_owner() {
        let pool = setup_test_db().await;
        let (project_id, user_id) = setup_project_with_member(&pool, MemberRole::Owner).await;

        let result = require_project_admin(&pool, user_id, project_id).await;
        assert!(matches!(result, Ok(MemberRole::Owner)));
    }

    #[tokio::test]
    async fn test_require_project_admin_allows_admin() {
        let pool = setup_test_db().await;
        let (project_id, user_id) = setup_project_with_member(&pool, MemberRole::Admin).await;

        let result = require_project_admin(&pool, user_id, project_id).await;
        assert!(matches!(result, Ok(MemberRole::Admin)));
    }

    #[tokio::test]
    async fn test_require_project_admin_forbids_member() {
        let pool = setup_test_db().await;
        let (project_id, user_id) = setup_project_with_member(&pool, MemberRole::Member).await;

        let result = require_project_admin(&pool, user_id, project_id).await;
        assert!(matches!(result, Err(AppError::Forbidden(_))));
    }

    #[tokio::test]
    async fn test_require_project_admin_forbids_non_member() {
        let pool = setup_test_db().await;
        let project = project_service::create_project(
            &pool,
            &CreateProjectRequest {
                name: "Test".to_string(),
                description: None,
            },
        )
        .await
        .unwrap();

        let result = require_project_admin(&pool, Uuid::new_v4(), project.id).await;
        assert!(matches!(result, Err(AppError::Forbidden(_))));
    }

    // --- require_project_owner ---

    #[tokio::test]
    async fn test_require_project_owner_allows_owner() {
        let pool = setup_test_db().await;
        let (project_id, user_id) = setup_project_with_member(&pool, MemberRole::Owner).await;

        let result = require_project_owner(&pool, user_id, project_id).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_require_project_owner_forbids_admin() {
        let pool = setup_test_db().await;
        let (project_id, user_id) = setup_project_with_member(&pool, MemberRole::Admin).await;

        let result = require_project_owner(&pool, user_id, project_id).await;
        assert!(matches!(result, Err(AppError::Forbidden(_))));
    }

    #[tokio::test]
    async fn test_require_project_owner_forbids_non_member() {
        let pool = setup_test_db().await;
        let project = project_service::create_project(
            &pool,
            &CreateProjectRequest {
                name: "Test".to_string(),
                description: None,
            },
        )
        .await
        .unwrap();

        let result = require_project_owner(&pool, Uuid::new_v4(), project.id).await;
        assert!(matches!(result, Err(AppError::Forbidden(_))));
    }

    // --- list_user_project_ids ---

    #[tokio::test]
    async fn test_list_user_project_ids_returns_member_projects() {
        let pool = setup_test_db().await;
        let user_id = Uuid::new_v4();

        let project1 = project_service::create_project(
            &pool,
            &CreateProjectRequest {
                name: "Project 1".to_string(),
                description: None,
            },
        )
        .await
        .unwrap();
        let project2 = project_service::create_project(
            &pool,
            &CreateProjectRequest {
                name: "Project 2".to_string(),
                description: None,
            },
        )
        .await
        .unwrap();
        // Third project - user is NOT a member
        project_service::create_project(
            &pool,
            &CreateProjectRequest {
                name: "Project 3".to_string(),
                description: None,
            },
        )
        .await
        .unwrap();

        member_service::add_member(
            &pool,
            project1.id,
            &AddMemberRequest {
                user_id,
                role: MemberRole::Owner,
            },
        )
        .await
        .unwrap();
        member_service::add_member(
            &pool,
            project2.id,
            &AddMemberRequest {
                user_id,
                role: MemberRole::Member,
            },
        )
        .await
        .unwrap();

        let project_ids = list_user_project_ids(&pool, user_id).await.unwrap();
        assert_eq!(project_ids.len(), 2);
        assert!(project_ids.contains(&project1.id));
        assert!(project_ids.contains(&project2.id));
    }

    #[tokio::test]
    async fn test_list_user_project_ids_returns_empty_for_no_memberships() {
        let pool = setup_test_db().await;
        let user_id = Uuid::new_v4();

        let project_ids = list_user_project_ids(&pool, user_id).await.unwrap();
        assert!(project_ids.is_empty());
    }

    // --- count_owners ---

    #[tokio::test]
    async fn test_count_owners_returns_correct_count() {
        let pool = setup_test_db().await;
        let project = project_service::create_project(
            &pool,
            &CreateProjectRequest {
                name: "Test".to_string(),
                description: None,
            },
        )
        .await
        .unwrap();

        let owner1 = Uuid::new_v4();
        let owner2 = Uuid::new_v4();
        let admin = Uuid::new_v4();

        member_service::add_member(
            &pool,
            project.id,
            &AddMemberRequest {
                user_id: owner1,
                role: MemberRole::Owner,
            },
        )
        .await
        .unwrap();
        member_service::add_member(
            &pool,
            project.id,
            &AddMemberRequest {
                user_id: owner2,
                role: MemberRole::Owner,
            },
        )
        .await
        .unwrap();
        member_service::add_member(
            &pool,
            project.id,
            &AddMemberRequest {
                user_id: admin,
                role: MemberRole::Admin,
            },
        )
        .await
        .unwrap();

        let count = count_owners(&pool, project.id).await.unwrap();
        assert_eq!(count, 2);
    }

    #[tokio::test]
    async fn test_count_owners_returns_zero_when_no_owners() {
        let pool = setup_test_db().await;
        let project = project_service::create_project(
            &pool,
            &CreateProjectRequest {
                name: "Test".to_string(),
                description: None,
            },
        )
        .await
        .unwrap();

        let count = count_owners(&pool, project.id).await.unwrap();
        assert_eq!(count, 0);
    }
}
