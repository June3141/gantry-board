use chrono::{DateTime, Utc};
use sqlx::{FromRow, SqlitePool};
use uuid::Uuid;

use crate::error::{AppError, AppResult};
use crate::models::github::GitHubIssueMapping;

#[derive(FromRow)]
struct MappingRow {
    id: String,
    task_id: String,
    github_link_id: String,
    github_issue_number: i64,
    github_issue_id: Option<i64>,
    last_local_update: Option<DateTime<Utc>>,
    last_remote_update: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
}

impl TryFrom<MappingRow> for GitHubIssueMapping {
    type Error = uuid::Error;

    fn try_from(row: MappingRow) -> Result<Self, Self::Error> {
        Ok(GitHubIssueMapping {
            id: row.id.parse()?,
            task_id: row.task_id.parse()?,
            github_link_id: row.github_link_id.parse()?,
            github_issue_number: row.github_issue_number,
            github_issue_id: row.github_issue_id,
            last_local_update: row.last_local_update,
            last_remote_update: row.last_remote_update,
            created_at: row.created_at,
        })
    }
}

fn row_to_mapping(row: MappingRow) -> AppResult<GitHubIssueMapping> {
    row.try_into()
        .map_err(|e: uuid::Error| AppError::Internal(e.to_string()))
}

/// Insert a new GitHub issue mapping.
pub async fn insert(
    pool: &SqlitePool,
    id: Uuid,
    task_id: Uuid,
    github_link_id: Uuid,
    issue_number: i64,
    issue_id: Option<i64>,
) -> AppResult<()> {
    sqlx::query(
        r#"
        INSERT INTO github_issue_mappings (id, task_id, github_link_id, github_issue_number, github_issue_id)
        VALUES ($1, $2, $3, $4, $5)
        "#,
    )
    .bind(id.to_string())
    .bind(task_id.to_string())
    .bind(github_link_id.to_string())
    .bind(issue_number)
    .bind(issue_id)
    .execute(pool)
    .await?;

    Ok(())
}

/// Find a mapping by its ID.
pub async fn find_by_id(pool: &SqlitePool, id: Uuid) -> AppResult<GitHubIssueMapping> {
    let row = sqlx::query_as::<_, MappingRow>(
        r#"
        SELECT id, task_id, github_link_id, github_issue_number, github_issue_id,
               last_local_update, last_remote_update, created_at
        FROM github_issue_mappings
        WHERE id = $1
        "#,
    )
    .bind(id.to_string())
    .fetch_optional(pool)
    .await?;

    match row {
        Some(r) => row_to_mapping(r),
        None => Err(AppError::NotFound(format!(
            "github issue mapping {} not found",
            id
        ))),
    }
}

/// Find a mapping by task ID (returns Option since a task may not have a mapping).
pub async fn find_by_task_id(
    pool: &SqlitePool,
    task_id: Uuid,
) -> AppResult<Option<GitHubIssueMapping>> {
    let row = sqlx::query_as::<_, MappingRow>(
        r#"
        SELECT id, task_id, github_link_id, github_issue_number, github_issue_id,
               last_local_update, last_remote_update, created_at
        FROM github_issue_mappings
        WHERE task_id = $1
        "#,
    )
    .bind(task_id.to_string())
    .fetch_optional(pool)
    .await?;

    row.map(row_to_mapping).transpose()
}

/// Find a mapping by GitHub link ID and issue number (returns Option).
pub async fn find_by_issue_number(
    pool: &SqlitePool,
    github_link_id: Uuid,
    issue_number: i64,
) -> AppResult<Option<GitHubIssueMapping>> {
    let row = sqlx::query_as::<_, MappingRow>(
        r#"
        SELECT id, task_id, github_link_id, github_issue_number, github_issue_id,
               last_local_update, last_remote_update, created_at
        FROM github_issue_mappings
        WHERE github_link_id = $1 AND github_issue_number = $2
        "#,
    )
    .bind(github_link_id.to_string())
    .bind(issue_number)
    .fetch_optional(pool)
    .await?;

    row.map(row_to_mapping).transpose()
}

/// Find all mappings for a given GitHub link.
pub async fn find_all_by_link(
    pool: &SqlitePool,
    github_link_id: Uuid,
) -> AppResult<Vec<GitHubIssueMapping>> {
    let rows = sqlx::query_as::<_, MappingRow>(
        r#"
        SELECT id, task_id, github_link_id, github_issue_number, github_issue_id,
               last_local_update, last_remote_update, created_at
        FROM github_issue_mappings
        WHERE github_link_id = $1
        "#,
    )
    .bind(github_link_id.to_string())
    .fetch_all(pool)
    .await?;

    rows.into_iter().map(row_to_mapping).collect()
}

/// Update the local and remote timestamps on a mapping.
pub async fn update_timestamps(
    pool: &SqlitePool,
    mapping_id: Uuid,
    last_local_update: Option<DateTime<Utc>>,
    last_remote_update: Option<DateTime<Utc>>,
) -> AppResult<()> {
    let result = sqlx::query(
        r#"
        UPDATE github_issue_mappings
        SET last_local_update = $2, last_remote_update = $3
        WHERE id = $1
        "#,
    )
    .bind(mapping_id.to_string())
    .bind(last_local_update)
    .bind(last_remote_update)
    .execute(pool)
    .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound(format!(
            "github issue mapping {} not found",
            mapping_id
        )));
    }

    Ok(())
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
    async fn test_insert_and_find_by_id() {
        let pool = setup_db().await;
        let project_id = create_project(&pool).await;
        let link_id = create_link(&pool, project_id).await;
        let task_id = create_task(&pool, project_id).await;

        let id = Uuid::new_v4();
        insert(&pool, id, task_id, link_id, 42, Some(12345))
            .await
            .unwrap();

        let mapping = find_by_id(&pool, id).await.unwrap();
        assert_eq!(mapping.id, id);
        assert_eq!(mapping.task_id, task_id);
        assert_eq!(mapping.github_link_id, link_id);
        assert_eq!(mapping.github_issue_number, 42);
        assert_eq!(mapping.github_issue_id, Some(12345));
    }

    #[tokio::test]
    async fn test_find_by_id_not_found() {
        let pool = setup_db().await;
        let result = find_by_id(&pool, Uuid::new_v4()).await;
        assert!(matches!(result, Err(AppError::NotFound(_))));
    }

    #[tokio::test]
    async fn test_find_by_task_id() {
        let pool = setup_db().await;
        let project_id = create_project(&pool).await;
        let link_id = create_link(&pool, project_id).await;
        let task_id = create_task(&pool, project_id).await;

        let id = Uuid::new_v4();
        insert(&pool, id, task_id, link_id, 42, Some(12345))
            .await
            .unwrap();

        let found = find_by_task_id(&pool, task_id).await.unwrap();
        assert!(found.is_some());
        let found = found.unwrap();
        assert_eq!(found.id, id);
        assert_eq!(found.task_id, task_id);
    }

    #[tokio::test]
    async fn test_find_by_task_id_not_found() {
        let pool = setup_db().await;
        let result = find_by_task_id(&pool, Uuid::new_v4()).await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_find_by_issue_number() {
        let pool = setup_db().await;
        let project_id = create_project(&pool).await;
        let link_id = create_link(&pool, project_id).await;
        let task_id = create_task(&pool, project_id).await;

        insert(&pool, Uuid::new_v4(), task_id, link_id, 99, None)
            .await
            .unwrap();

        let found = find_by_issue_number(&pool, link_id, 99).await.unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().github_issue_number, 99);

        // Different issue number -> None
        let not_found = find_by_issue_number(&pool, link_id, 100).await.unwrap();
        assert!(not_found.is_none());
    }

    #[tokio::test]
    async fn test_find_all_by_link() {
        let pool = setup_db().await;
        let project_id = create_project(&pool).await;
        let link_id = create_link(&pool, project_id).await;
        let task1 = create_task(&pool, project_id).await;
        let task2 = create_task(&pool, project_id).await;

        insert(&pool, Uuid::new_v4(), task1, link_id, 1, None)
            .await
            .unwrap();
        insert(&pool, Uuid::new_v4(), task2, link_id, 2, None)
            .await
            .unwrap();

        let mappings = find_all_by_link(&pool, link_id).await.unwrap();
        assert_eq!(mappings.len(), 2);
    }

    #[tokio::test]
    async fn test_find_all_by_link_empty() {
        let pool = setup_db().await;
        let mappings = find_all_by_link(&pool, Uuid::new_v4()).await.unwrap();
        assert!(mappings.is_empty());
    }

    #[tokio::test]
    async fn test_update_timestamps() {
        let pool = setup_db().await;
        let project_id = create_project(&pool).await;
        let link_id = create_link(&pool, project_id).await;
        let task_id = create_task(&pool, project_id).await;

        let id = Uuid::new_v4();
        insert(&pool, id, task_id, link_id, 10, None).await.unwrap();

        let mapping = find_by_id(&pool, id).await.unwrap();
        assert!(mapping.last_local_update.is_none());
        assert!(mapping.last_remote_update.is_none());

        let now = Utc::now();
        update_timestamps(&pool, id, Some(now), Some(now))
            .await
            .unwrap();

        let updated = find_by_id(&pool, id).await.unwrap();
        assert!(updated.last_local_update.is_some());
        assert!(updated.last_remote_update.is_some());
    }

    #[tokio::test]
    async fn test_update_timestamps_not_found() {
        let pool = setup_db().await;
        let result = update_timestamps(&pool, Uuid::new_v4(), None, None).await;
        assert!(matches!(result, Err(AppError::NotFound(_))));
    }
}
