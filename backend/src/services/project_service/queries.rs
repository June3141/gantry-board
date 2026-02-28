use sqlx::SqlitePool;
use uuid::Uuid;

use crate::error::{AppError, AppResult};
use crate::models::project::Project;
use crate::repositories::project_repository;

pub async fn get_project(pool: &SqlitePool, id: Uuid) -> AppResult<Project> {
    project_repository::find_by_id(pool, id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("project {} not found", id)))
}

pub async fn list_projects(pool: &SqlitePool) -> AppResult<Vec<Project>> {
    project_repository::find_all(pool).await
}

pub async fn list_projects_paginated(
    pool: &SqlitePool,
    limit: i64,
    offset: i64,
) -> AppResult<(Vec<Project>, i64)> {
    project_repository::find_all_paginated(pool, limit, offset).await
}

pub async fn list_projects_for_user(pool: &SqlitePool, user_id: Uuid) -> AppResult<Vec<Project>> {
    project_repository::find_all_for_user(pool, user_id).await
}

pub async fn list_projects_for_user_paginated(
    pool: &SqlitePool,
    user_id: Uuid,
    limit: i64,
    offset: i64,
) -> AppResult<(Vec<Project>, i64)> {
    project_repository::find_all_for_user_paginated(pool, user_id, limit, offset).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::project::CreateProjectRequest;
    use crate::services::project_service::create_project;
    use crate::test_helpers::setup_test_db;

    #[tokio::test]
    async fn test_get_project_returns_existing_project() {
        let pool = setup_test_db().await;
        let req = CreateProjectRequest {
            name: "Test Project".to_string(),
            description: None,
            repository_path: None,
        };
        let created = create_project(&pool, &req)
            .await
            .expect("Failed to create project");

        let found = get_project(&pool, created.id)
            .await
            .expect("Failed to get project");

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
            repository_path: None,
        };
        let req2 = CreateProjectRequest {
            name: "Project 2".to_string(),
            description: None,
            repository_path: None,
        };
        create_project(&pool, &req1)
            .await
            .expect("Failed to create project 1");
        create_project(&pool, &req2)
            .await
            .expect("Failed to create project 2");

        let projects = list_projects(&pool).await.expect("Failed to list projects");

        assert_eq!(projects.len(), 2);
    }

    #[tokio::test]
    async fn test_list_projects_paginated_returns_total_and_data() {
        let pool = setup_test_db().await;

        for i in 0..5 {
            create_project(
                &pool,
                &CreateProjectRequest {
                    name: format!("Project {}", i),
                    description: None,
                    repository_path: None,
                },
            )
            .await
            .expect("Failed to create project");
        }

        let (projects, total) = list_projects_paginated(&pool, 2, 0)
            .await
            .expect("Failed to list projects paginated");

        assert_eq!(projects.len(), 2);
        assert_eq!(total, 5);
    }

    #[tokio::test]
    async fn test_list_projects_paginated_respects_offset() {
        let pool = setup_test_db().await;

        for i in 0..5 {
            create_project(
                &pool,
                &CreateProjectRequest {
                    name: format!("Project {}", i),
                    description: None,
                    repository_path: None,
                },
            )
            .await
            .expect("Failed to create project");
        }

        let (projects, total) = list_projects_paginated(&pool, 2, 3)
            .await
            .expect("Failed to list projects paginated");

        assert_eq!(projects.len(), 2);
        assert_eq!(total, 5);
    }

    #[tokio::test]
    async fn test_get_project_returns_repository_path() {
        let pool = setup_test_db().await;
        let req = CreateProjectRequest {
            name: "Repo Project".to_string(),
            description: None,
            repository_path: Some("/opt/repos/project".to_string()),
        };
        let created = create_project(&pool, &req)
            .await
            .expect("Failed to create project");

        let found = get_project(&pool, created.id)
            .await
            .expect("Failed to get project");

        assert_eq!(
            found.repository_path,
            Some("/opt/repos/project".to_string())
        );
    }
}
