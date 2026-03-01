use sqlx::SqlitePool;
use uuid::Uuid;

use crate::error::AppResult;
use crate::models::github::{GitHubPullRequest, LinkedPr};
use crate::repositories::github_pr_repository;

/// Upsert a pull request linked to a task.
/// Inserts a new record or updates an existing one (matched by github_link_id + pr_number + task_id).
#[tracing::instrument(skip(pool, pr), fields(pr_number = pr.pr_number))]
pub async fn upsert_pr(
    pool: &SqlitePool,
    github_link_id: Uuid,
    task_id: Uuid,
    pr: &LinkedPr,
) -> AppResult<GitHubPullRequest> {
    let id = Uuid::new_v4();
    let state = match pr.state.as_str() {
        "closed" => "closed",
        _ => "open",
    };
    github_pr_repository::upsert(
        pool,
        id,
        github_link_id,
        task_id,
        pr.pr_number as i64,
        &pr.title,
        &pr.url,
        state,
        pr.is_merged,
        pr.author.as_deref(),
    )
    .await
}

/// List all pull requests linked to a task.
#[tracing::instrument(skip(pool))]
pub async fn list_prs_for_task(
    pool: &SqlitePool,
    task_id: Uuid,
) -> AppResult<Vec<GitHubPullRequest>> {
    github_pr_repository::find_all_by_task(pool, task_id).await
}

/// Find task IDs linked to a specific PR number within a github_link.
pub async fn find_task_ids_by_pr(
    pool: &SqlitePool,
    github_link_id: Uuid,
    pr_number: i64,
) -> AppResult<Vec<Uuid>> {
    github_pr_repository::find_task_ids_by_pr(pool, github_link_id, pr_number).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::github::PrState;
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

    fn make_linked_pr(pr_number: u64) -> LinkedPr {
        LinkedPr {
            pr_number,
            title: format!("PR #{pr_number}"),
            url: format!("https://github.com/owner/repo/pull/{pr_number}"),
            state: "open".to_string(),
            is_merged: false,
            author: Some("octocat".to_string()),
        }
    }

    #[tokio::test]
    async fn test_upsert_pr_creates_new_record() {
        let pool = setup_db().await;
        let project_id = create_project(&pool).await;
        let link_id = create_link(&pool, project_id).await;
        let task_id = create_task(&pool, project_id).await;

        let pr = make_linked_pr(42);
        let result = upsert_pr(&pool, link_id, task_id, &pr).await.unwrap();

        assert_eq!(result.pr_number, 42);
        assert_eq!(result.title, "PR #42");
        assert_eq!(result.state, PrState::Open);
        assert!(!result.is_merged);
        assert_eq!(result.author.as_deref(), Some("octocat"));
        assert_eq!(result.github_link_id, link_id);
        assert_eq!(result.task_id, task_id);
    }

    #[tokio::test]
    async fn test_upsert_pr_updates_existing_record() {
        let pool = setup_db().await;
        let project_id = create_project(&pool).await;
        let link_id = create_link(&pool, project_id).await;
        let task_id = create_task(&pool, project_id).await;

        let pr = make_linked_pr(42);
        let first = upsert_pr(&pool, link_id, task_id, &pr).await.unwrap();

        // Update with merged state
        let updated_pr = LinkedPr {
            pr_number: 42,
            title: "PR #42 (merged)".to_string(),
            url: "https://github.com/owner/repo/pull/42".to_string(),
            state: "closed".to_string(),
            is_merged: true,
            author: Some("octocat".to_string()),
        };
        let second = upsert_pr(&pool, link_id, task_id, &updated_pr)
            .await
            .unwrap();

        assert_eq!(first.id, second.id);
        assert_eq!(second.title, "PR #42 (merged)");
        assert_eq!(second.state, PrState::Closed);
        assert!(second.is_merged);
    }

    #[tokio::test]
    async fn test_list_prs_empty() {
        let pool = setup_db().await;
        let project_id = create_project(&pool).await;
        let task_id = create_task(&pool, project_id).await;

        let prs = list_prs_for_task(&pool, task_id).await.unwrap();
        assert!(prs.is_empty());
    }

    #[tokio::test]
    async fn test_list_prs_returns_linked_prs() {
        let pool = setup_db().await;
        let project_id = create_project(&pool).await;
        let link_id = create_link(&pool, project_id).await;
        let task_id = create_task(&pool, project_id).await;

        upsert_pr(&pool, link_id, task_id, &make_linked_pr(1))
            .await
            .unwrap();
        upsert_pr(&pool, link_id, task_id, &make_linked_pr(2))
            .await
            .unwrap();

        let prs = list_prs_for_task(&pool, task_id).await.unwrap();
        assert_eq!(prs.len(), 2);
    }

    #[tokio::test]
    async fn test_list_prs_does_not_include_other_tasks() {
        let pool = setup_db().await;
        let project_id = create_project(&pool).await;
        let link_id = create_link(&pool, project_id).await;
        let task_a = create_task(&pool, project_id).await;
        let task_b = create_task(&pool, project_id).await;

        upsert_pr(&pool, link_id, task_a, &make_linked_pr(10))
            .await
            .unwrap();
        upsert_pr(&pool, link_id, task_b, &make_linked_pr(20))
            .await
            .unwrap();

        let prs_a = list_prs_for_task(&pool, task_a).await.unwrap();
        assert_eq!(prs_a.len(), 1);
        assert_eq!(prs_a[0].pr_number, 10);

        let prs_b = list_prs_for_task(&pool, task_b).await.unwrap();
        assert_eq!(prs_b.len(), 1);
        assert_eq!(prs_b[0].pr_number, 20);
    }
}
