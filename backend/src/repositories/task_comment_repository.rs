use chrono::{DateTime, Utc};
use sqlx::{FromRow, SqlitePool};
use uuid::Uuid;

use crate::error::{AppError, AppResult};
use crate::models::task_comment::TaskComment;

#[derive(FromRow)]
struct CommentRow {
    id: String,
    task_id: String,
    user_id: String,
    user_name: String,
    content: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl TryFrom<CommentRow> for TaskComment {
    type Error = uuid::Error;

    fn try_from(row: CommentRow) -> Result<Self, Self::Error> {
        Ok(TaskComment {
            id: row.id.parse()?,
            task_id: row.task_id.parse()?,
            user_id: row.user_id.parse()?,
            user_name: row.user_name,
            content: row.content,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
    }
}

fn row_to_comment(row: CommentRow) -> AppResult<TaskComment> {
    row.try_into()
        .map_err(|e: uuid::Error| AppError::Internal(e.to_string()))
}

/// Insert a new task comment.
pub async fn insert(
    pool: &SqlitePool,
    id: Uuid,
    task_id: Uuid,
    user_id: Uuid,
    content: &str,
    now: DateTime<Utc>,
) -> AppResult<()> {
    sqlx::query(
        r#"
        INSERT INTO task_comments (id, task_id, user_id, content, created_at, updated_at)
        VALUES ($1, $2, $3, $4, $5, $6)
        "#,
    )
    .bind(id.to_string())
    .bind(task_id.to_string())
    .bind(user_id.to_string())
    .bind(content)
    .bind(now)
    .bind(now)
    .execute(pool)
    .await?;

    Ok(())
}

/// Find a comment by its ID (with user name via JOIN).
pub async fn find_by_id(pool: &SqlitePool, comment_id: Uuid) -> AppResult<TaskComment> {
    let row = sqlx::query_as::<_, CommentRow>(
        r#"
        SELECT c.id, c.task_id, c.user_id, u.name as user_name,
               c.content, c.created_at, c.updated_at
        FROM task_comments c
        JOIN users u ON c.user_id = u.id
        WHERE c.id = $1
        "#,
    )
    .bind(comment_id.to_string())
    .fetch_optional(pool)
    .await?;

    match row {
        Some(r) => row_to_comment(r),
        None => Err(AppError::NotFound(format!(
            "comment {} not found",
            comment_id
        ))),
    }
}

/// Find all comments for a task, ordered by created_at ASC.
pub async fn find_all_by_task(pool: &SqlitePool, task_id: Uuid) -> AppResult<Vec<TaskComment>> {
    let rows = sqlx::query_as::<_, CommentRow>(
        r#"
        SELECT c.id, c.task_id, c.user_id, u.name as user_name,
               c.content, c.created_at, c.updated_at
        FROM task_comments c
        JOIN users u ON c.user_id = u.id
        WHERE c.task_id = $1
        ORDER BY c.created_at ASC
        "#,
    )
    .bind(task_id.to_string())
    .fetch_all(pool)
    .await?;

    rows.into_iter().map(row_to_comment).collect()
}

/// Update a comment's content.
pub async fn update_content(
    pool: &SqlitePool,
    comment_id: Uuid,
    content: &str,
    now: DateTime<Utc>,
) -> AppResult<()> {
    let result = sqlx::query(
        r#"
        UPDATE task_comments
        SET content = $1, updated_at = $2
        WHERE id = $3
        "#,
    )
    .bind(content)
    .bind(now)
    .bind(comment_id.to_string())
    .execute(pool)
    .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound(format!(
            "comment {} not found",
            comment_id
        )));
    }

    Ok(())
}

/// Delete a comment. Returns the number of rows affected.
pub async fn delete(pool: &SqlitePool, comment_id: Uuid) -> AppResult<u64> {
    let result = sqlx::query("DELETE FROM task_comments WHERE id = $1")
        .bind(comment_id.to_string())
        .execute(pool)
        .await?;

    Ok(result.rows_affected())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::project::CreateProjectRequest;
    use crate::models::task::{CreateTaskRequest, TaskPriority, TaskStatus};
    use crate::models::user::RegisterRequest;
    use crate::services::{project_service, task_service, user_service};
    use crate::test_helpers::setup_test_db;

    async fn setup_task_and_user(pool: &SqlitePool) -> (Uuid, Uuid) {
        let user = user_service::create_user(
            pool,
            &RegisterRequest {
                email: "alice@test.com".to_string(),
                name: "Alice".to_string(),
                password: "password123".to_string(),
            },
        )
        .await
        .unwrap();

        let project = project_service::create_project(
            pool,
            &CreateProjectRequest {
                name: "Test Project".to_string(),
                description: None,
                repository_path: None,
            },
        )
        .await
        .unwrap();

        let task = task_service::create_task(
            pool,
            &CreateTaskRequest {
                project_id: project.id,
                title: "Test Task".to_string(),
                description: None,
                status: Some(TaskStatus::Todo),
                priority: Some(TaskPriority::Medium),
                parent_id: None,
                assigned_to: None,
            },
        )
        .await
        .unwrap();

        (task.id, user.id)
    }

    #[tokio::test]
    async fn test_insert_and_find_by_id() {
        let pool = setup_test_db().await;
        let (task_id, user_id) = setup_task_and_user(&pool).await;

        let id = Uuid::new_v4();
        let now = Utc::now();
        insert(&pool, id, task_id, user_id, "Hello, world!", now)
            .await
            .unwrap();

        let comment = find_by_id(&pool, id).await.unwrap();
        assert_eq!(comment.id, id);
        assert_eq!(comment.task_id, task_id);
        assert_eq!(comment.user_id, user_id);
        assert_eq!(comment.content, "Hello, world!");
        assert_eq!(comment.user_name, "Alice");
    }

    #[tokio::test]
    async fn test_find_by_id_not_found() {
        let pool = setup_test_db().await;
        let result = find_by_id(&pool, Uuid::new_v4()).await;
        assert!(matches!(result, Err(AppError::NotFound(_))));
    }

    #[tokio::test]
    async fn test_find_all_by_task_returns_chronological_order() {
        let pool = setup_test_db().await;
        let (task_id, user_id) = setup_task_and_user(&pool).await;

        let now = Utc::now();
        let id1 = Uuid::new_v4();
        let id2 = Uuid::new_v4();

        insert(&pool, id1, task_id, user_id, "First", now)
            .await
            .unwrap();
        insert(
            &pool,
            id2,
            task_id,
            user_id,
            "Second",
            now + chrono::Duration::seconds(1),
        )
        .await
        .unwrap();

        let comments = find_all_by_task(&pool, task_id).await.unwrap();
        assert_eq!(comments.len(), 2);
        assert_eq!(comments[0].content, "First");
        assert_eq!(comments[1].content, "Second");
    }

    #[tokio::test]
    async fn test_find_all_by_task_empty() {
        let pool = setup_test_db().await;
        let comments = find_all_by_task(&pool, Uuid::new_v4()).await.unwrap();
        assert!(comments.is_empty());
    }

    #[tokio::test]
    async fn test_update_content() {
        let pool = setup_test_db().await;
        let (task_id, user_id) = setup_task_and_user(&pool).await;

        let id = Uuid::new_v4();
        let now = Utc::now();
        insert(&pool, id, task_id, user_id, "Original", now)
            .await
            .unwrap();

        let later = now + chrono::Duration::seconds(1);
        update_content(&pool, id, "Updated", later).await.unwrap();

        let comment = find_by_id(&pool, id).await.unwrap();
        assert_eq!(comment.content, "Updated");
    }

    #[tokio::test]
    async fn test_update_content_not_found() {
        let pool = setup_test_db().await;
        let result = update_content(&pool, Uuid::new_v4(), "content", Utc::now()).await;
        assert!(matches!(result, Err(AppError::NotFound(_))));
    }

    #[tokio::test]
    async fn test_delete() {
        let pool = setup_test_db().await;
        let (task_id, user_id) = setup_task_and_user(&pool).await;

        let id = Uuid::new_v4();
        let now = Utc::now();
        insert(&pool, id, task_id, user_id, "To delete", now)
            .await
            .unwrap();

        let rows_affected = delete(&pool, id).await.unwrap();
        assert_eq!(rows_affected, 1);

        let result = find_by_id(&pool, id).await;
        assert!(matches!(result, Err(AppError::NotFound(_))));
    }

    #[tokio::test]
    async fn test_delete_not_found() {
        let pool = setup_test_db().await;
        let rows_affected = delete(&pool, Uuid::new_v4()).await.unwrap();
        assert_eq!(rows_affected, 0);
    }
}
