use sqlx::SqlitePool;
use uuid::Uuid;

use crate::error::{AppError, AppResult};
use crate::models::project::{CreateProjectRequest, Project, UpdateProjectRequest};

pub async fn create_project(
    _pool: &SqlitePool,
    _req: &CreateProjectRequest,
) -> AppResult<Project> {
    Err(AppError::Internal(anyhow::anyhow!("not implemented")))
}

pub async fn get_project(_pool: &SqlitePool, _id: Uuid) -> AppResult<Project> {
    Err(AppError::Internal(anyhow::anyhow!("not implemented")))
}

pub async fn list_projects(_pool: &SqlitePool) -> AppResult<Vec<Project>> {
    Err(AppError::Internal(anyhow::anyhow!("not implemented")))
}

pub async fn update_project(
    _pool: &SqlitePool,
    _id: Uuid,
    _req: &UpdateProjectRequest,
) -> AppResult<Project> {
    Err(AppError::Internal(anyhow::anyhow!("not implemented")))
}

pub async fn delete_project(_pool: &SqlitePool, _id: Uuid) -> AppResult<()> {
    Err(AppError::Internal(anyhow::anyhow!("not implemented")))
}

#[cfg(test)]
mod tests {
    use super::*;
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

    #[tokio::test]
    async fn test_create_project_saves_to_db_and_returns() {
        let pool = setup_test_db().await;
        let req = CreateProjectRequest {
            name: "Test Project".to_string(),
            description: Some("A test project".to_string()),
        };

        let project = create_project(&pool, &req).await.expect("Failed to create project");

        assert_eq!(project.name, "Test Project");
        assert_eq!(project.description, Some("A test project".to_string()));
        assert!(!project.id.is_nil());
    }

    #[tokio::test]
    async fn test_get_project_returns_existing_project() {
        let pool = setup_test_db().await;
        let req = CreateProjectRequest {
            name: "Test Project".to_string(),
            description: None,
        };
        let created = create_project(&pool, &req).await.expect("Failed to create project");

        let found = get_project(&pool, created.id).await.expect("Failed to get project");

        assert_eq!(found.id, created.id);
        assert_eq!(found.name, "Test Project");
    }

    #[tokio::test]
    async fn test_get_project_returns_not_found_for_nonexistent() {
        let pool = setup_test_db().await;
        let random_id = Uuid::new_v4();

        let result = get_project(&pool, random_id).await;

        assert!(matches!(result, Err(AppError::NotFound(_))));
    }

    #[tokio::test]
    async fn test_list_projects_returns_empty_when_no_projects() {
        let pool = setup_test_db().await;

        let projects = list_projects(&pool).await.expect("Failed to list projects");

        assert!(projects.is_empty());
    }

    #[tokio::test]
    async fn test_list_projects_returns_multiple_projects() {
        let pool = setup_test_db().await;
        let req1 = CreateProjectRequest {
            name: "Project 1".to_string(),
            description: None,
        };
        let req2 = CreateProjectRequest {
            name: "Project 2".to_string(),
            description: None,
        };
        create_project(&pool, &req1).await.expect("Failed to create project 1");
        create_project(&pool, &req2).await.expect("Failed to create project 2");

        let projects = list_projects(&pool).await.expect("Failed to list projects");

        assert_eq!(projects.len(), 2);
    }

    #[tokio::test]
    async fn test_update_project_changes_name() {
        let pool = setup_test_db().await;
        let req = CreateProjectRequest {
            name: "Original Name".to_string(),
            description: None,
        };
        let created = create_project(&pool, &req).await.expect("Failed to create project");

        let update_req = UpdateProjectRequest {
            name: Some("Updated Name".to_string()),
            description: None,
        };
        let updated =
            update_project(&pool, created.id, &update_req).await.expect("Failed to update project");

        assert_eq!(updated.name, "Updated Name");
    }

    #[tokio::test]
    async fn test_update_project_changes_description() {
        let pool = setup_test_db().await;
        let req = CreateProjectRequest {
            name: "Test Project".to_string(),
            description: Some("Original description".to_string()),
        };
        let created = create_project(&pool, &req).await.expect("Failed to create project");

        let update_req = UpdateProjectRequest {
            name: None,
            description: Some("Updated description".to_string()),
        };
        let updated =
            update_project(&pool, created.id, &update_req).await.expect("Failed to update project");

        assert_eq!(updated.description, Some("Updated description".to_string()));
    }

    #[tokio::test]
    async fn test_delete_project_removes_from_db() {
        let pool = setup_test_db().await;
        let req = CreateProjectRequest {
            name: "To Be Deleted".to_string(),
            description: None,
        };
        let created = create_project(&pool, &req).await.expect("Failed to create project");

        delete_project(&pool, created.id).await.expect("Failed to delete project");

        let result = get_project(&pool, created.id).await;
        assert!(matches!(result, Err(AppError::NotFound(_))));
    }

    #[tokio::test]
    async fn test_delete_nonexistent_project_returns_not_found() {
        let pool = setup_test_db().await;
        let random_id = Uuid::new_v4();

        let result = delete_project(&pool, random_id).await;

        assert!(matches!(result, Err(AppError::NotFound(_))));
    }
}
