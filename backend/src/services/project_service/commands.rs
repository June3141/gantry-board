use chrono::Utc;
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::error::{AppError, AppResult};
use crate::models::project::{CreateProjectRequest, Project, UpdateProjectRequest};
use crate::repositories::project_repository;

use super::queries::get_project;

#[tracing::instrument(skip(pool, req))]
pub async fn create_project(pool: &SqlitePool, req: &CreateProjectRequest) -> AppResult<Project> {
    let id = Uuid::new_v4();
    let now = Utc::now();

    project_repository::insert(
        pool,
        id,
        &req.name,
        req.description.as_deref(),
        req.repository_path.as_deref(),
        now,
    )
    .await?;

    Ok(Project {
        id,
        name: req.name.clone(),
        description: req.description.clone(),
        repository_path: req.repository_path.clone(),
        created_at: now,
        updated_at: now,
    })
}

#[tracing::instrument(skip(pool, req), fields(project_id = %id))]
pub async fn update_project(
    pool: &SqlitePool,
    id: Uuid,
    req: &UpdateProjectRequest,
) -> AppResult<Project> {
    let existing = get_project(pool, id).await?;
    let now = Utc::now();

    let name = req.name.as_ref().unwrap_or(&existing.name);
    // NOTE: With Option<String>, None means "don't update" (keeps existing value).
    // To support explicitly setting description to NULL, use Option<Option<String>>
    // or a custom enum in UpdateProjectRequest. This is acceptable for Phase 1.
    let description = req.description.as_ref().or(existing.description.as_ref());
    let repository_path = req
        .repository_path
        .as_ref()
        .or(existing.repository_path.as_ref());

    project_repository::update(
        pool,
        id,
        name,
        description.map(|s| s.as_str()),
        repository_path.map(|s| s.as_str()),
        now,
    )
    .await?;

    Ok(Project {
        id,
        name: name.clone(),
        description: description.cloned(),
        repository_path: repository_path.cloned(),
        created_at: existing.created_at,
        updated_at: now,
    })
}

/// Validate that a repository path points to a valid git repository.
pub fn validate_repository_path(path: &str) -> AppResult<()> {
    let trimmed = path.trim();
    if trimmed.is_empty() {
        return Err(AppError::Validation("repository path is empty".to_string()));
    }
    let p = std::path::Path::new(trimmed);
    if !p.exists() {
        return Err(AppError::Validation(format!(
            "repository path does not exist: {trimmed}"
        )));
    }
    if !p.is_dir() {
        return Err(AppError::Validation(format!(
            "repository path is not a directory: {trimmed}"
        )));
    }
    git2::Repository::open(p).map_err(|_| {
        AppError::Validation(format!("path is not a valid git repository: {trimmed}"))
    })?;
    Ok(())
}

#[tracing::instrument(skip(pool), fields(project_id = %id))]
pub async fn delete_project(pool: &SqlitePool, id: Uuid) -> AppResult<()> {
    let rows = project_repository::delete(pool, id).await?;

    if rows == 0 {
        return Err(AppError::NotFound(format!("project {} not found", id)));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers::setup_test_db;

    #[tokio::test]
    async fn test_create_project_saves_to_db_and_returns() {
        let pool = setup_test_db().await;
        let req = CreateProjectRequest {
            name: "Test Project".to_string(),
            description: Some("A test project".to_string()),
            repository_path: None,
        };

        let project = create_project(&pool, &req)
            .await
            .expect("Failed to create project");

        assert_eq!(project.name, "Test Project");
        assert_eq!(project.description, Some("A test project".to_string()));
        assert!(!project.id.is_nil());
    }

    #[tokio::test]
    async fn test_update_project_changes_name() {
        let pool = setup_test_db().await;
        let req = CreateProjectRequest {
            name: "Original Name".to_string(),
            description: None,
            repository_path: None,
        };
        let created = create_project(&pool, &req)
            .await
            .expect("Failed to create project");

        let update_req = UpdateProjectRequest {
            name: Some("Updated Name".to_string()),
            description: None,
            repository_path: None,
        };
        let updated = update_project(&pool, created.id, &update_req)
            .await
            .expect("Failed to update project");

        assert_eq!(updated.name, "Updated Name");
    }

    #[tokio::test]
    async fn test_update_project_changes_description() {
        let pool = setup_test_db().await;
        let req = CreateProjectRequest {
            name: "Test Project".to_string(),
            description: Some("Original description".to_string()),
            repository_path: None,
        };
        let created = create_project(&pool, &req)
            .await
            .expect("Failed to create project");

        let update_req = UpdateProjectRequest {
            name: None,
            description: Some("Updated description".to_string()),
            repository_path: None,
        };
        let updated = update_project(&pool, created.id, &update_req)
            .await
            .expect("Failed to update project");

        assert_eq!(updated.description, Some("Updated description".to_string()));
    }

    #[tokio::test]
    async fn test_delete_project_removes_from_db() {
        let pool = setup_test_db().await;
        let req = CreateProjectRequest {
            name: "To Be Deleted".to_string(),
            description: None,
            repository_path: None,
        };
        let created = create_project(&pool, &req)
            .await
            .expect("Failed to create project");

        delete_project(&pool, created.id)
            .await
            .expect("Failed to delete project");

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

    #[tokio::test]
    async fn test_create_project_with_repository_path() {
        let pool = setup_test_db().await;
        let req = CreateProjectRequest {
            name: "Repo Project".to_string(),
            description: None,
            repository_path: Some("/home/user/my-repo".to_string()),
        };

        let project = create_project(&pool, &req)
            .await
            .expect("Failed to create project");

        assert_eq!(
            project.repository_path,
            Some("/home/user/my-repo".to_string())
        );
    }

    #[tokio::test]
    async fn test_create_project_without_repository_path() {
        let pool = setup_test_db().await;
        let req = CreateProjectRequest {
            name: "No Repo Project".to_string(),
            description: None,
            repository_path: None,
        };

        let project = create_project(&pool, &req)
            .await
            .expect("Failed to create project");

        assert!(project.repository_path.is_none());
    }

    #[tokio::test]
    async fn test_update_project_sets_repository_path() {
        let pool = setup_test_db().await;
        let req = CreateProjectRequest {
            name: "Test Project".to_string(),
            description: None,
            repository_path: None,
        };
        let created = create_project(&pool, &req)
            .await
            .expect("Failed to create project");

        let update_req = UpdateProjectRequest {
            name: None,
            description: None,
            repository_path: Some("/new/repo/path".to_string()),
        };
        let updated = update_project(&pool, created.id, &update_req)
            .await
            .expect("Failed to update project");

        assert_eq!(updated.repository_path, Some("/new/repo/path".to_string()));
    }

    #[tokio::test]
    async fn test_update_project_preserves_repository_path_when_not_provided() {
        let pool = setup_test_db().await;
        let req = CreateProjectRequest {
            name: "Test Project".to_string(),
            description: None,
            repository_path: Some("/existing/path".to_string()),
        };
        let created = create_project(&pool, &req)
            .await
            .expect("Failed to create project");

        let update_req = UpdateProjectRequest {
            name: Some("Updated Name".to_string()),
            description: None,
            repository_path: None,
        };
        let updated = update_project(&pool, created.id, &update_req)
            .await
            .expect("Failed to update project");

        assert_eq!(updated.repository_path, Some("/existing/path".to_string()));
    }

    #[tokio::test]
    async fn test_validate_repository_path_rejects_nonexistent() {
        let result = validate_repository_path("/nonexistent/path/that/does/not/exist");
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_validate_repository_path_rejects_non_directory() {
        let dir = tempfile::TempDir::new().expect("Failed to create temp dir");
        let file_path = dir.path().join("not-a-dir.txt");
        std::fs::write(&file_path, "test").expect("Failed to create file");

        let result = validate_repository_path(file_path.to_str().unwrap());
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_validate_repository_path_rejects_non_git_repo() {
        let dir = tempfile::TempDir::new().expect("Failed to create temp dir");

        let result = validate_repository_path(dir.path().to_str().unwrap());
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_validate_repository_path_accepts_valid_git_repo() {
        let dir = tempfile::TempDir::new().expect("Failed to create temp dir");
        git2::Repository::init(dir.path()).expect("Failed to init repo");

        let result = validate_repository_path(dir.path().to_str().unwrap());
        assert!(result.is_ok());
    }
}
