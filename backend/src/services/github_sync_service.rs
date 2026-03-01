use chrono::{DateTime, Utc};
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::error::AppResult;
use crate::models::github::GitHubIssueMapping;
use crate::repositories::github_sync_repository;

/// Create a new issue mapping.
pub async fn create_mapping(
    pool: &SqlitePool,
    task_id: Uuid,
    github_link_id: Uuid,
    issue_number: i64,
    issue_id: Option<i64>,
) -> AppResult<GitHubIssueMapping> {
    let id = Uuid::new_v4();
    github_sync_repository::insert(pool, id, task_id, github_link_id, issue_number, issue_id)
        .await?;
    github_sync_repository::find_by_id(pool, id).await
}

/// Get a mapping by task ID.
pub async fn get_mapping_by_task_id(
    pool: &SqlitePool,
    task_id: Uuid,
) -> AppResult<Option<GitHubIssueMapping>> {
    github_sync_repository::find_by_task_id(pool, task_id).await
}

/// Get a mapping by GitHub link ID and issue number.
pub async fn get_mapping_by_issue_number(
    pool: &SqlitePool,
    github_link_id: Uuid,
    issue_number: i64,
) -> AppResult<Option<GitHubIssueMapping>> {
    github_sync_repository::find_by_issue_number(pool, github_link_id, issue_number).await
}

/// List all mappings for a given GitHub link.
pub async fn list_mappings_by_link(
    pool: &SqlitePool,
    github_link_id: Uuid,
) -> AppResult<Vec<GitHubIssueMapping>> {
    github_sync_repository::find_all_by_link(pool, github_link_id).await
}

/// Update the local and remote timestamps on a mapping.
pub async fn update_mapping_timestamps(
    pool: &SqlitePool,
    mapping_id: Uuid,
    last_local_update: Option<DateTime<Utc>>,
    last_remote_update: Option<DateTime<Utc>>,
) -> AppResult<()> {
    github_sync_repository::update_timestamps(
        pool,
        mapping_id,
        last_local_update,
        last_remote_update,
    )
    .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::sqlite::SqlitePoolOptions;

    async fn setup_db() -> SqlitePool {
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

    /// Helper: create a project and return its ID.
    async fn create_project(pool: &SqlitePool) -> Uuid {
        let id = Uuid::new_v4();
        sqlx::query("INSERT INTO projects (id, name) VALUES ($1, $2)")
            .bind(id.to_string())
            .bind("test-project")
            .execute(pool)
            .await
            .unwrap();
        id
    }

    /// Helper: create a github_link and return its ID.
    async fn create_link(pool: &SqlitePool, project_id: Uuid) -> Uuid {
        let id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO github_links (id, project_id, repo_owner, repo_name) VALUES ($1, $2, $3, $4)",
        )
        .bind(id.to_string())
        .bind(project_id.to_string())
        .bind("owner")
        .bind("repo")
        .execute(pool)
        .await
        .unwrap();
        id
    }

    /// Helper: create a task and return its ID.
    async fn create_task(pool: &SqlitePool, project_id: Uuid) -> Uuid {
        let id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO tasks (id, project_id, title, status, priority, position) VALUES ($1, $2, $3, $4, $5, $6)",
        )
        .bind(id.to_string())
        .bind(project_id.to_string())
        .bind("Test Task")
        .bind("todo")
        .bind("medium")
        .bind(0)
        .execute(pool)
        .await
        .unwrap();
        id
    }

    #[tokio::test]
    async fn test_create_and_get_mapping_by_task_id() {
        let pool = setup_db().await;
        let project_id = create_project(&pool).await;
        let link_id = create_link(&pool, project_id).await;
        let task_id = create_task(&pool, project_id).await;

        let mapping = create_mapping(&pool, task_id, link_id, 42, Some(12345))
            .await
            .unwrap();

        assert_eq!(mapping.task_id, task_id);
        assert_eq!(mapping.github_link_id, link_id);
        assert_eq!(mapping.github_issue_number, 42);
        assert_eq!(mapping.github_issue_id, Some(12345));

        let found = get_mapping_by_task_id(&pool, task_id).await.unwrap();
        assert!(found.is_some());
        let found = found.unwrap();
        assert_eq!(found.id, mapping.id);
    }

    #[tokio::test]
    async fn test_get_mapping_by_task_id_not_found() {
        let pool = setup_db().await;
        let result = get_mapping_by_task_id(&pool, Uuid::new_v4()).await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_get_mapping_by_issue_number() {
        let pool = setup_db().await;
        let project_id = create_project(&pool).await;
        let link_id = create_link(&pool, project_id).await;
        let task_id = create_task(&pool, project_id).await;

        create_mapping(&pool, task_id, link_id, 99, None)
            .await
            .unwrap();

        let found = get_mapping_by_issue_number(&pool, link_id, 99)
            .await
            .unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().github_issue_number, 99);

        // Different issue number → None
        let not_found = get_mapping_by_issue_number(&pool, link_id, 100)
            .await
            .unwrap();
        assert!(not_found.is_none());
    }

    #[tokio::test]
    async fn test_list_mappings_by_link() {
        let pool = setup_db().await;
        let project_id = create_project(&pool).await;
        let link_id = create_link(&pool, project_id).await;
        let task1 = create_task(&pool, project_id).await;
        let task2 = create_task(&pool, project_id).await;

        create_mapping(&pool, task1, link_id, 1, None)
            .await
            .unwrap();
        create_mapping(&pool, task2, link_id, 2, None)
            .await
            .unwrap();

        let mappings = list_mappings_by_link(&pool, link_id).await.unwrap();
        assert_eq!(mappings.len(), 2);
    }

    #[tokio::test]
    async fn test_list_mappings_empty() {
        let pool = setup_db().await;
        let mappings = list_mappings_by_link(&pool, Uuid::new_v4()).await.unwrap();
        assert!(mappings.is_empty());
    }

    #[tokio::test]
    async fn test_update_mapping_timestamps() {
        let pool = setup_db().await;
        let project_id = create_project(&pool).await;
        let link_id = create_link(&pool, project_id).await;
        let task_id = create_task(&pool, project_id).await;

        let mapping = create_mapping(&pool, task_id, link_id, 10, None)
            .await
            .unwrap();
        assert!(mapping.last_local_update.is_none());
        assert!(mapping.last_remote_update.is_none());

        let now = Utc::now();
        update_mapping_timestamps(&pool, mapping.id, Some(now), Some(now))
            .await
            .unwrap();

        let updated = get_mapping_by_task_id(&pool, task_id)
            .await
            .unwrap()
            .unwrap();
        assert!(updated.last_local_update.is_some());
        assert!(updated.last_remote_update.is_some());
    }

    #[tokio::test]
    async fn test_update_mapping_not_found() {
        let pool = setup_db().await;
        let result = update_mapping_timestamps(&pool, Uuid::new_v4(), None, None).await;
        assert!(result.is_err());
    }
}
