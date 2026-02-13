use sqlx::SqlitePool;
use uuid::Uuid;

use crate::error::{AppError, AppResult};
use crate::models::project::MemberRole;
use crate::models::task::Task;
use crate::services::{project_service, task_service};

/// Fetch a task and verify the user is a member of its project.
/// Returns the task on success.
pub async fn authorize_task(pool: &SqlitePool, user_id: Uuid, task_id: Uuid) -> AppResult<Task> {
    todo!()
}

/// Verify a project exists and the user is a member.
pub async fn authorize_project(
    pool: &SqlitePool,
    user_id: Uuid,
    project_id: Uuid,
) -> AppResult<()> {
    todo!()
}

/// Check if user is a member of the project (any role).
/// Returns the member's role or Forbidden.
pub async fn require_project_member(
    pool: &SqlitePool,
    user_id: Uuid,
    project_id: Uuid,
) -> AppResult<MemberRole> {
    // Nil UUID bypass for auth_disabled mode — compiled out of release builds
    #[cfg(debug_assertions)]
    if user_id.is_nil() {
        return Ok(MemberRole::Owner);
    }

    let row = sqlx::query_scalar::<_, String>(
        r#"
        SELECT role FROM project_members
        WHERE project_id = $1 AND user_id = $2
        "#,
    )
    .bind(project_id.to_string())
    .bind(user_id.to_string())
    .fetch_optional(pool)
    .await?;

    match row {
        Some(role_str) => {
            let role: MemberRole = serde_json::from_value(serde_json::Value::String(role_str))
                .map_err(|e| AppError::Internal(e.to_string()))?;
            Ok(role)
        }
        None => Err(AppError::Forbidden(format!(
            "user {} is not a member of project {}",
            user_id, project_id
        ))),
    }
}

/// Check if user has Owner or Admin role in the project.
/// Returns the member's role or Forbidden.
pub async fn require_project_admin(
    pool: &SqlitePool,
    user_id: Uuid,
    project_id: Uuid,
) -> AppResult<MemberRole> {
    let role = require_project_member(pool, user_id, project_id).await?;
    match role {
        MemberRole::Owner | MemberRole::Admin => Ok(role),
        MemberRole::Member => Err(AppError::Forbidden(
            "insufficient permissions: admin or owner role required".to_string(),
        )),
    }
}

/// Check if user is Owner of the project.
/// Returns Ok(()) or Forbidden.
pub async fn require_project_owner(
    pool: &SqlitePool,
    user_id: Uuid,
    project_id: Uuid,
) -> AppResult<()> {
    let role = require_project_member(pool, user_id, project_id).await?;
    match role {
        MemberRole::Owner => Ok(()),
        _ => Err(AppError::Forbidden(
            "insufficient permissions: owner role required".to_string(),
        )),
    }
}

/// List project IDs the user is a member of.
pub async fn list_user_project_ids(pool: &SqlitePool, user_id: Uuid) -> AppResult<Vec<Uuid>> {
    let rows = sqlx::query_scalar::<_, String>(
        r#"
        SELECT project_id FROM project_members
        WHERE user_id = $1
        "#,
    )
    .bind(user_id.to_string())
    .fetch_all(pool)
    .await?;

    rows.into_iter()
        .map(|s| {
            s.parse()
                .map_err(|e: uuid::Error| AppError::Internal(e.to_string()))
        })
        .collect()
}

/// Count owners in a project.
pub async fn count_owners(pool: &SqlitePool, project_id: Uuid) -> AppResult<i64> {
    let count = sqlx::query_scalar::<_, i32>(
        r#"
        SELECT COUNT(*) FROM project_members
        WHERE project_id = $1 AND role = 'owner'
        "#,
    )
    .bind(project_id.to_string())
    .fetch_one(pool)
    .await?;

    Ok(count as i64)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::project::AddMemberRequest;
    use crate::models::user::RegisterRequest;
    use crate::services::{member_service, project_service, user_service};
    use crate::test_helpers::setup_test_db;

    use crate::models::project::CreateProjectRequest;

    static USER_COUNTER: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);

    async fn create_test_user(pool: &SqlitePool) -> Uuid {
        let n = USER_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let req = RegisterRequest {
            email: format!("user{n}@test.com"),
            name: format!("User {n}"),
            password: "password123".to_string(),
        };
        user_service::create_user(pool, &req)
            .await
            .expect("Failed to create user")
            .id
    }

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
        let user_id = create_test_user(pool).await;
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

    #[cfg(debug_assertions)]
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
        let user_id = create_test_user(&pool).await;

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

    // --- authorize_task ---

    #[tokio::test]
    async fn test_authorize_task_returns_task_for_member() {
        let pool = setup_test_db().await;
        let (project_id, user_id) = setup_project_with_member(&pool, MemberRole::Member).await;
        let task = task_service::create_task(
            &pool,
            &crate::models::task::CreateTaskRequest {
                project_id,
                title: "Test Task".to_string(),
                description: None,
                status: None,
                priority: None,
                parent_id: None,
                assigned_to: None,
            },
        )
        .await
        .unwrap();

        let result = authorize_task(&pool, user_id, task.id).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().id, task.id);
    }

    #[tokio::test]
    async fn test_authorize_task_returns_forbidden_for_non_member() {
        let pool = setup_test_db().await;
        let (project_id, _owner) = setup_project_with_member(&pool, MemberRole::Owner).await;
        let task = task_service::create_task(
            &pool,
            &crate::models::task::CreateTaskRequest {
                project_id,
                title: "Test Task".to_string(),
                description: None,
                status: None,
                priority: None,
                parent_id: None,
                assigned_to: None,
            },
        )
        .await
        .unwrap();

        let non_member = create_test_user(&pool).await;
        let result = authorize_task(&pool, non_member, task.id).await;
        assert!(matches!(result, Err(AppError::Forbidden(_))));
    }

    #[tokio::test]
    async fn test_authorize_task_returns_not_found_for_missing_task() {
        let pool = setup_test_db().await;
        let user_id = create_test_user(&pool).await;

        let result = authorize_task(&pool, user_id, Uuid::new_v4()).await;
        assert!(matches!(result, Err(AppError::NotFound(_))));
    }

    // --- authorize_project ---

    #[tokio::test]
    async fn test_authorize_project_allows_member() {
        let pool = setup_test_db().await;
        let (project_id, user_id) = setup_project_with_member(&pool, MemberRole::Member).await;

        let result = authorize_project(&pool, user_id, project_id).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_authorize_project_returns_forbidden_for_non_member() {
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

        let non_member = create_test_user(&pool).await;
        let result = authorize_project(&pool, non_member, project.id).await;
        assert!(matches!(result, Err(AppError::Forbidden(_))));
    }

    #[tokio::test]
    async fn test_authorize_project_returns_not_found_for_missing_project() {
        let pool = setup_test_db().await;
        let user_id = create_test_user(&pool).await;

        let result = authorize_project(&pool, user_id, Uuid::new_v4()).await;
        assert!(matches!(result, Err(AppError::NotFound(_))));
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

        let owner1 = create_test_user(&pool).await;
        let owner2 = create_test_user(&pool).await;
        let admin = create_test_user(&pool).await;

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
