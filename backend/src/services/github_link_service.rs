use chrono::{DateTime, Utc};
use sqlx::{FromRow, SqlitePool};
use uuid::Uuid;

use crate::error::{AppError, AppResult};
use crate::models::github::{CreateGitHubLinkRequest, GitHubLink};

#[derive(FromRow)]
struct GitHubLinkRow {
    id: String,
    project_id: String,
    repo_owner: String,
    repo_name: String,
    sync_enabled: bool,
    last_synced_at: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl TryFrom<GitHubLinkRow> for GitHubLink {
    type Error = uuid::Error;

    fn try_from(row: GitHubLinkRow) -> Result<Self, Self::Error> {
        Ok(GitHubLink {
            id: row.id.parse()?,
            project_id: row.project_id.parse()?,
            repo_owner: row.repo_owner,
            repo_name: row.repo_name,
            sync_enabled: row.sync_enabled,
            last_synced_at: row.last_synced_at,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
    }
}

#[tracing::instrument(skip(pool, req), fields(project_id = %project_id))]
pub async fn create_github_link(
    pool: &SqlitePool,
    project_id: Uuid,
    req: &CreateGitHubLinkRequest,
) -> AppResult<GitHubLink> {
    let id = Uuid::new_v4();
    sqlx::query(
        r#"
        INSERT INTO github_links (id, project_id, repo_owner, repo_name)
        VALUES ($1, $2, $3, $4)
        "#,
    )
    .bind(id.to_string())
    .bind(project_id.to_string())
    .bind(&req.repo_owner)
    .bind(&req.repo_name)
    .execute(pool)
    .await?;

    get_github_link(pool, project_id).await
}

pub async fn get_github_link(pool: &SqlitePool, project_id: Uuid) -> AppResult<GitHubLink> {
    let row = sqlx::query_as::<_, GitHubLinkRow>(
        r#"
        SELECT id, project_id, repo_owner, repo_name, sync_enabled,
               last_synced_at, created_at, updated_at
        FROM github_links
        WHERE project_id = $1
        "#,
    )
    .bind(project_id.to_string())
    .fetch_optional(pool)
    .await?;

    match row {
        Some(r) => Ok(r
            .try_into()
            .map_err(|e: uuid::Error| AppError::Internal(e.to_string()))?),
        None => Err(AppError::NotFound(format!(
            "github link for project {} not found",
            project_id
        ))),
    }
}

#[tracing::instrument(skip(pool), fields(project_id = %project_id))]
pub async fn delete_github_link(pool: &SqlitePool, project_id: Uuid) -> AppResult<()> {
    let result = sqlx::query("DELETE FROM github_links WHERE project_id = $1")
        .bind(project_id.to_string())
        .execute(pool)
        .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound(format!(
            "github link for project {} not found",
            project_id
        )));
    }

    Ok(())
}

pub async fn list_sync_enabled(pool: &SqlitePool) -> AppResult<Vec<GitHubLink>> {
    let rows = sqlx::query_as::<_, GitHubLinkRow>(
        r#"
        SELECT id, project_id, repo_owner, repo_name, sync_enabled,
               last_synced_at, created_at, updated_at
        FROM github_links
        WHERE sync_enabled = 1
        "#,
    )
    .fetch_all(pool)
    .await?;

    rows.into_iter()
        .map(|r| {
            r.try_into()
                .map_err(|e: uuid::Error| AppError::Internal(e.to_string()))
        })
        .collect()
}

/// Find a GitHub link by repo owner/name (for webhook routing).
pub async fn find_by_repo(
    pool: &SqlitePool,
    repo_owner: &str,
    repo_name: &str,
) -> AppResult<Option<GitHubLink>> {
    let row = sqlx::query_as::<_, GitHubLinkRow>(
        r#"
        SELECT id, project_id, repo_owner, repo_name, sync_enabled,
               last_synced_at, created_at, updated_at
        FROM github_links
        WHERE repo_owner = $1 AND repo_name = $2 AND sync_enabled = 1
        "#,
    )
    .bind(repo_owner)
    .bind(repo_name)
    .fetch_optional(pool)
    .await?;

    match row {
        Some(r) => {
            Ok(Some(r.try_into().map_err(|e: uuid::Error| {
                AppError::Internal(e.to_string())
            })?))
        }
        None => Ok(None),
    }
}

#[tracing::instrument(skip(pool), fields(project_id = %project_id))]
pub async fn update_last_synced(pool: &SqlitePool, project_id: Uuid) -> AppResult<()> {
    let result = sqlx::query(
        r#"
        UPDATE github_links
        SET last_synced_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
            updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
        WHERE project_id = $1
        "#,
    )
    .bind(project_id.to_string())
    .execute(pool)
    .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound(format!(
            "github link for project {} not found",
            project_id
        )));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
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
        assert!(result.is_err());
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
        assert!(result.is_err());
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
        assert!(result.is_err());
    }
}
