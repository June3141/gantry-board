use chrono::{DateTime, Utc};
use sqlx::{FromRow, SqlitePool};
use uuid::Uuid;

use crate::error::{AppError, AppResult};
use crate::models::github::{GitHubPullRequest, PrState};

#[derive(FromRow)]
struct PrRow {
    id: String,
    github_link_id: String,
    task_id: String,
    pr_number: i64,
    title: String,
    url: String,
    state: String,
    is_merged: bool,
    author: Option<String>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl TryFrom<PrRow> for GitHubPullRequest {
    type Error = uuid::Error;

    fn try_from(row: PrRow) -> Result<Self, Self::Error> {
        let state = match row.state.as_str() {
            "closed" => PrState::Closed,
            _ => PrState::Open,
        };
        Ok(GitHubPullRequest {
            id: row.id.parse()?,
            github_link_id: row.github_link_id.parse()?,
            task_id: row.task_id.parse()?,
            pr_number: row.pr_number,
            title: row.title,
            url: row.url,
            state,
            is_merged: row.is_merged,
            author: row.author,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
    }
}

fn row_to_pr(row: PrRow) -> AppResult<GitHubPullRequest> {
    row.try_into()
        .map_err(|e: uuid::Error| AppError::Internal(e.to_string()))
}

/// Upsert a pull request linked to a task.
/// Inserts a new record or updates an existing one (matched by github_link_id + pr_number + task_id).
#[allow(clippy::too_many_arguments)]
pub async fn upsert(
    pool: &SqlitePool,
    id: Uuid,
    github_link_id: Uuid,
    task_id: Uuid,
    pr_number: i64,
    title: &str,
    url: &str,
    state: &str,
    is_merged: bool,
    author: Option<&str>,
) -> AppResult<GitHubPullRequest> {
    sqlx::query(
        r#"
        INSERT INTO github_pull_requests (id, github_link_id, task_id, pr_number, title, url, state, is_merged, author)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
        ON CONFLICT (github_link_id, pr_number, task_id) DO UPDATE SET
            title = excluded.title,
            url = excluded.url,
            state = excluded.state,
            is_merged = excluded.is_merged,
            author = excluded.author,
            updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
        "#,
    )
    .bind(id.to_string())
    .bind(github_link_id.to_string())
    .bind(task_id.to_string())
    .bind(pr_number)
    .bind(title)
    .bind(url)
    .bind(state)
    .bind(is_merged)
    .bind(author)
    .execute(pool)
    .await?;

    let row = sqlx::query_as::<_, PrRow>(
        r#"
        SELECT id, github_link_id, task_id, pr_number, title, url, state, is_merged, author, created_at, updated_at
        FROM github_pull_requests
        WHERE github_link_id = $1 AND pr_number = $2 AND task_id = $3
        "#,
    )
    .bind(github_link_id.to_string())
    .bind(pr_number)
    .bind(task_id.to_string())
    .fetch_one(pool)
    .await?;

    row_to_pr(row)
}

/// Find a specific pull request by link, PR number, and task.
pub async fn find_by_link_and_pr(
    pool: &SqlitePool,
    github_link_id: Uuid,
    pr_number: i64,
    task_id: Uuid,
) -> AppResult<Option<GitHubPullRequest>> {
    let row = sqlx::query_as::<_, PrRow>(
        r#"
        SELECT id, github_link_id, task_id, pr_number, title, url, state, is_merged, author, created_at, updated_at
        FROM github_pull_requests
        WHERE github_link_id = $1 AND pr_number = $2 AND task_id = $3
        "#,
    )
    .bind(github_link_id.to_string())
    .bind(pr_number)
    .bind(task_id.to_string())
    .fetch_optional(pool)
    .await?;

    row.map(row_to_pr).transpose()
}

/// List all pull requests linked to a task.
pub async fn find_all_by_task(
    pool: &SqlitePool,
    task_id: Uuid,
) -> AppResult<Vec<GitHubPullRequest>> {
    let rows = sqlx::query_as::<_, PrRow>(
        r#"
        SELECT id, github_link_id, task_id, pr_number, title, url, state, is_merged, author, created_at, updated_at
        FROM github_pull_requests
        WHERE task_id = $1
        ORDER BY pr_number
        "#,
    )
    .bind(task_id.to_string())
    .fetch_all(pool)
    .await?;

    rows.into_iter().map(row_to_pr).collect()
}

/// Find task IDs linked to a specific PR number within a github_link.
pub async fn find_task_ids_by_pr(
    pool: &SqlitePool,
    github_link_id: Uuid,
    pr_number: i64,
) -> AppResult<Vec<Uuid>> {
    let rows: Vec<(String,)> = sqlx::query_as(
        "SELECT task_id FROM github_pull_requests WHERE github_link_id = $1 AND pr_number = $2",
    )
    .bind(github_link_id.to_string())
    .bind(pr_number)
    .fetch_all(pool)
    .await?;

    rows.into_iter()
        .map(|(id,)| {
            id.parse::<Uuid>()
                .map_err(|e| AppError::Internal(e.to_string()))
        })
        .collect()
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
    async fn test_upsert_creates_new_record() {
        let pool = setup_db().await;
        let project_id = create_project(&pool).await;
        let link_id = create_link(&pool, project_id).await;
        let task_id = create_task(&pool, project_id).await;

        let id = Uuid::new_v4();
        let result = upsert(
            &pool,
            id,
            link_id,
            task_id,
            42,
            "PR #42",
            "https://github.com/owner/repo/pull/42",
            "open",
            false,
            Some("octocat"),
        )
        .await
        .unwrap();

        assert_eq!(result.pr_number, 42);
        assert_eq!(result.title, "PR #42");
        assert_eq!(result.state, PrState::Open);
        assert!(!result.is_merged);
        assert_eq!(result.author.as_deref(), Some("octocat"));
        assert_eq!(result.github_link_id, link_id);
        assert_eq!(result.task_id, task_id);
    }

    #[tokio::test]
    async fn test_upsert_updates_existing_record() {
        let pool = setup_db().await;
        let project_id = create_project(&pool).await;
        let link_id = create_link(&pool, project_id).await;
        let task_id = create_task(&pool, project_id).await;

        let id = Uuid::new_v4();
        let first = upsert(
            &pool,
            id,
            link_id,
            task_id,
            42,
            "PR #42",
            "https://github.com/owner/repo/pull/42",
            "open",
            false,
            Some("octocat"),
        )
        .await
        .unwrap();

        // Update with merged state
        let second = upsert(
            &pool,
            Uuid::new_v4(),
            link_id,
            task_id,
            42,
            "PR #42 (merged)",
            "https://github.com/owner/repo/pull/42",
            "closed",
            true,
            Some("octocat"),
        )
        .await
        .unwrap();

        assert_eq!(first.id, second.id);
        assert_eq!(second.title, "PR #42 (merged)");
        assert_eq!(second.state, PrState::Closed);
        assert!(second.is_merged);
    }

    #[tokio::test]
    async fn test_find_by_link_and_pr() {
        let pool = setup_db().await;
        let project_id = create_project(&pool).await;
        let link_id = create_link(&pool, project_id).await;
        let task_id = create_task(&pool, project_id).await;

        upsert(
            &pool,
            Uuid::new_v4(),
            link_id,
            task_id,
            42,
            "PR #42",
            "https://github.com/owner/repo/pull/42",
            "open",
            false,
            None,
        )
        .await
        .unwrap();

        let found = find_by_link_and_pr(&pool, link_id, 42, task_id)
            .await
            .unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().pr_number, 42);

        // Non-existent PR number
        let not_found = find_by_link_and_pr(&pool, link_id, 999, task_id)
            .await
            .unwrap();
        assert!(not_found.is_none());
    }

    #[tokio::test]
    async fn test_find_all_by_task_empty() {
        let pool = setup_db().await;
        let project_id = create_project(&pool).await;
        let task_id = create_task(&pool, project_id).await;

        let prs = find_all_by_task(&pool, task_id).await.unwrap();
        assert!(prs.is_empty());
    }

    #[tokio::test]
    async fn test_find_all_by_task_returns_linked_prs() {
        let pool = setup_db().await;
        let project_id = create_project(&pool).await;
        let link_id = create_link(&pool, project_id).await;
        let task_id = create_task(&pool, project_id).await;

        upsert(
            &pool,
            Uuid::new_v4(),
            link_id,
            task_id,
            1,
            "PR #1",
            "https://github.com/owner/repo/pull/1",
            "open",
            false,
            Some("octocat"),
        )
        .await
        .unwrap();
        upsert(
            &pool,
            Uuid::new_v4(),
            link_id,
            task_id,
            2,
            "PR #2",
            "https://github.com/owner/repo/pull/2",
            "open",
            false,
            Some("octocat"),
        )
        .await
        .unwrap();

        let prs = find_all_by_task(&pool, task_id).await.unwrap();
        assert_eq!(prs.len(), 2);
    }

    #[tokio::test]
    async fn test_find_all_by_task_does_not_include_other_tasks() {
        let pool = setup_db().await;
        let project_id = create_project(&pool).await;
        let link_id = create_link(&pool, project_id).await;
        let task_a = create_task(&pool, project_id).await;
        let task_b = create_task(&pool, project_id).await;

        upsert(
            &pool,
            Uuid::new_v4(),
            link_id,
            task_a,
            10,
            "PR #10",
            "https://github.com/owner/repo/pull/10",
            "open",
            false,
            None,
        )
        .await
        .unwrap();
        upsert(
            &pool,
            Uuid::new_v4(),
            link_id,
            task_b,
            20,
            "PR #20",
            "https://github.com/owner/repo/pull/20",
            "open",
            false,
            None,
        )
        .await
        .unwrap();

        let prs_a = find_all_by_task(&pool, task_a).await.unwrap();
        assert_eq!(prs_a.len(), 1);
        assert_eq!(prs_a[0].pr_number, 10);

        let prs_b = find_all_by_task(&pool, task_b).await.unwrap();
        assert_eq!(prs_b.len(), 1);
        assert_eq!(prs_b[0].pr_number, 20);
    }

    #[tokio::test]
    async fn test_find_task_ids_by_pr() {
        let pool = setup_db().await;
        let project_id = create_project(&pool).await;
        let link_id = create_link(&pool, project_id).await;
        let task_a = create_task(&pool, project_id).await;
        let task_b = create_task(&pool, project_id).await;

        // Same PR number linked to two different tasks
        upsert(
            &pool,
            Uuid::new_v4(),
            link_id,
            task_a,
            42,
            "PR #42",
            "https://github.com/owner/repo/pull/42",
            "open",
            false,
            None,
        )
        .await
        .unwrap();
        upsert(
            &pool,
            Uuid::new_v4(),
            link_id,
            task_b,
            42,
            "PR #42",
            "https://github.com/owner/repo/pull/42",
            "open",
            false,
            None,
        )
        .await
        .unwrap();

        let task_ids = find_task_ids_by_pr(&pool, link_id, 42).await.unwrap();
        assert_eq!(task_ids.len(), 2);
        assert!(task_ids.contains(&task_a));
        assert!(task_ids.contains(&task_b));
    }

    #[tokio::test]
    async fn test_find_task_ids_by_pr_empty() {
        let pool = setup_db().await;
        let task_ids = find_task_ids_by_pr(&pool, Uuid::new_v4(), 999)
            .await
            .unwrap();
        assert!(task_ids.is_empty());
    }
}
