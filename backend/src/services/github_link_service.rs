use sqlx::SqlitePool;
use uuid::Uuid;

use crate::error::AppResult;
use crate::models::github::{CreateGitHubLinkRequest, GitHubLink};
use crate::repositories::github_link_repository;

#[tracing::instrument(skip(pool, req), fields(project_id = %project_id))]
pub async fn create_github_link(
    pool: &SqlitePool,
    project_id: Uuid,
    req: &CreateGitHubLinkRequest,
) -> AppResult<GitHubLink> {
    let id = Uuid::new_v4();
    github_link_repository::insert(pool, id, project_id, &req.repo_owner, &req.repo_name).await?;

    get_github_link(pool, project_id).await
}

pub async fn get_github_link(pool: &SqlitePool, project_id: Uuid) -> AppResult<GitHubLink> {
    github_link_repository::find_by_project(pool, project_id).await
}

#[tracing::instrument(skip(pool), fields(project_id = %project_id))]
pub async fn delete_github_link(pool: &SqlitePool, project_id: Uuid) -> AppResult<()> {
    github_link_repository::delete_by_project(pool, project_id).await
}

pub async fn list_sync_enabled(pool: &SqlitePool) -> AppResult<Vec<GitHubLink>> {
    github_link_repository::find_all_sync_enabled(pool).await
}

/// Find a GitHub link by repo owner/name (for webhook routing).
pub async fn find_by_repo(
    pool: &SqlitePool,
    repo_owner: &str,
    repo_name: &str,
) -> AppResult<Option<GitHubLink>> {
    github_link_repository::find_by_repo(pool, repo_owner, repo_name).await
}

#[tracing::instrument(skip(pool), fields(project_id = %project_id))]
pub async fn update_last_synced(pool: &SqlitePool, project_id: Uuid) -> AppResult<()> {
    github_link_repository::update_last_synced(pool, project_id).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::AppError;
    use crate::models::project::CreateProjectRequest;
    use crate::services::project_service;
    use crate::test_helpers::setup_test_db;

    async fn create_test_project(pool: &SqlitePool) -> Uuid {
        let req = CreateProjectRequest {
            name: "Test Project".to_string(),
            description: None,
            repository_path: None,
        };
        project_service::create_project(pool, &req)
            .await
            .expect("create project")
            .id
    }

    #[tokio::test]
    async fn test_create_and_get_github_link() {
        let pool = setup_test_db().await;
        let project_id = create_test_project(&pool).await;

        let req = CreateGitHubLinkRequest {
            repo_owner: "octocat".to_string(),
            repo_name: "hello-world".to_string(),
        };

        let link = create_github_link(&pool, project_id, &req).await.unwrap();
        assert_eq!(link.project_id, project_id);
        assert_eq!(link.repo_owner, "octocat");
        assert_eq!(link.repo_name, "hello-world");
        assert!(link.sync_enabled);

        let fetched = get_github_link(&pool, project_id).await.unwrap();
        assert_eq!(fetched.id, link.id);
    }

    #[tokio::test]
    async fn test_get_github_link_not_found() {
        let pool = setup_test_db().await;
        let fake_id = Uuid::new_v4();

        let result = get_github_link(&pool, fake_id).await;
        assert!(matches!(result, Err(AppError::NotFound(_))));
    }

    #[tokio::test]
    async fn test_delete_github_link() {
        let pool = setup_test_db().await;
        let project_id = create_test_project(&pool).await;

        let req = CreateGitHubLinkRequest {
            repo_owner: "octocat".to_string(),
            repo_name: "hello-world".to_string(),
        };
        create_github_link(&pool, project_id, &req).await.unwrap();

        delete_github_link(&pool, project_id).await.unwrap();

        let result = get_github_link(&pool, project_id).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_delete_github_link_not_found() {
        let pool = setup_test_db().await;
        let fake_id = Uuid::new_v4();

        let result = delete_github_link(&pool, fake_id).await;
        assert!(matches!(result, Err(AppError::NotFound(_))));
    }

    #[tokio::test]
    async fn test_list_sync_enabled() {
        let pool = setup_test_db().await;
        let project_id = create_test_project(&pool).await;

        // Initially empty
        let links = list_sync_enabled(&pool).await.unwrap();
        assert!(links.is_empty());

        // Create a link (sync_enabled defaults to true)
        let req = CreateGitHubLinkRequest {
            repo_owner: "octocat".to_string(),
            repo_name: "hello-world".to_string(),
        };
        create_github_link(&pool, project_id, &req).await.unwrap();

        let links = list_sync_enabled(&pool).await.unwrap();
        assert_eq!(links.len(), 1);
    }

    #[tokio::test]
    async fn test_find_by_repo() {
        let pool = setup_test_db().await;
        let project_id = create_test_project(&pool).await;

        let req = CreateGitHubLinkRequest {
            repo_owner: "octocat".to_string(),
            repo_name: "hello-world".to_string(),
        };
        create_github_link(&pool, project_id, &req).await.unwrap();

        let found = find_by_repo(&pool, "octocat", "hello-world").await.unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().project_id, project_id);

        let not_found = find_by_repo(&pool, "octocat", "nonexistent").await.unwrap();
        assert!(not_found.is_none());
    }

    #[tokio::test]
    async fn test_update_last_synced() {
        let pool = setup_test_db().await;
        let project_id = create_test_project(&pool).await;

        let req = CreateGitHubLinkRequest {
            repo_owner: "octocat".to_string(),
            repo_name: "hello-world".to_string(),
        };
        create_github_link(&pool, project_id, &req).await.unwrap();

        // Before update, last_synced_at should be None
        let link = get_github_link(&pool, project_id).await.unwrap();
        assert!(link.last_synced_at.is_none());

        update_last_synced(&pool, project_id).await.unwrap();

        let link = get_github_link(&pool, project_id).await.unwrap();
        assert!(link.last_synced_at.is_some());
    }

    #[tokio::test]
    async fn test_update_last_synced_not_found() {
        let pool = setup_test_db().await;
        let fake_id = Uuid::new_v4();

        let result = update_last_synced(&pool, fake_id).await;
        assert!(matches!(result, Err(AppError::NotFound(_))));
    }
}
