use chrono::{DateTime, Utc};
use sqlx::prelude::FromRow;
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::error::{AppError, AppResult};
use crate::models::task_comment::{CreateCommentRequest, TaskComment, UpdateCommentRequest};

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

pub async fn create_comment(
    pool: &SqlitePool,
    task_id: Uuid,
    user_id: Uuid,
    req: &CreateCommentRequest,
) -> AppResult<TaskComment> {
    let id = Uuid::new_v4();
    let now = Utc::now();

    sqlx::query(
        r#"
        INSERT INTO task_comments (id, task_id, user_id, content, created_at, updated_at)
        VALUES ($1, $2, $3, $4, $5, $6)
        "#,
    )
    .bind(id.to_string())
    .bind(task_id.to_string())
    .bind(user_id.to_string())
    .bind(&req.content)
    .bind(now)
    .bind(now)
    .execute(pool)
    .await?;

    get_comment(pool, id).await
}

pub async fn get_comment(pool: &SqlitePool, comment_id: Uuid) -> AppResult<TaskComment> {
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

    row.map(|r| r.try_into())
        .transpose()
        .map_err(|e: uuid::Error| AppError::Internal(e.to_string()))?
        .ok_or_else(|| AppError::NotFound(format!("comment {} not found", comment_id)))
}

pub async fn list_comments(pool: &SqlitePool, task_id: Uuid) -> AppResult<Vec<TaskComment>> {
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

    rows.into_iter()
        .map(|r| r.try_into())
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e: uuid::Error| AppError::Internal(e.to_string()))
}

pub async fn update_comment(
    pool: &SqlitePool,
    comment_id: Uuid,
    user_id: Uuid,
    req: &UpdateCommentRequest,
) -> AppResult<TaskComment> {
    let existing = get_comment(pool, comment_id).await?;

    if existing.user_id != user_id {
        return Err(AppError::Forbidden(
            "only the comment author can edit this comment".to_string(),
        ));
    }

    let now = Utc::now();
    sqlx::query(
        r#"
        UPDATE task_comments
        SET content = $1, updated_at = $2
        WHERE id = $3
        "#,
    )
    .bind(&req.content)
    .bind(now)
    .bind(comment_id.to_string())
    .execute(pool)
    .await?;

    get_comment(pool, comment_id).await
}

pub async fn delete_comment(pool: &SqlitePool, comment_id: Uuid, user_id: Uuid) -> AppResult<()> {
    let existing = get_comment(pool, comment_id).await?;

    if existing.user_id != user_id {
        return Err(AppError::Forbidden(
            "only the comment author can delete this comment".to_string(),
        ));
    }

    sqlx::query("DELETE FROM task_comments WHERE id = $1")
        .bind(comment_id.to_string())
        .execute(pool)
        .await?;

    Ok(())
}

/// Delete a comment regardless of author (for admin use).
pub async fn delete_comment_admin(pool: &SqlitePool, comment_id: Uuid) -> AppResult<()> {
    let result = sqlx::query("DELETE FROM task_comments WHERE id = $1")
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
    async fn test_create_comment() {
        let pool = setup_test_db().await;
        let (task_id, user_id) = setup_task_and_user(&pool).await;

        let req = CreateCommentRequest {
            content: "Hello, world!".to_string(),
        };
        let comment = create_comment(&pool, task_id, user_id, &req).await.unwrap();

        assert_eq!(comment.task_id, task_id);
        assert_eq!(comment.user_id, user_id);
        assert_eq!(comment.content, "Hello, world!");
        assert_eq!(comment.user_name, "Alice");
    }

    #[tokio::test]
    async fn test_list_comments_returns_chronological_order() {
        let pool = setup_test_db().await;
        let (task_id, user_id) = setup_task_and_user(&pool).await;

        create_comment(
            &pool,
            task_id,
            user_id,
            &CreateCommentRequest {
                content: "First".to_string(),
            },
        )
        .await
        .unwrap();

        create_comment(
            &pool,
            task_id,
            user_id,
            &CreateCommentRequest {
                content: "Second".to_string(),
            },
        )
        .await
        .unwrap();

        let comments = list_comments(&pool, task_id).await.unwrap();
        assert_eq!(comments.len(), 2);
        assert_eq!(comments[0].content, "First");
        assert_eq!(comments[1].content, "Second");
    }

    #[tokio::test]
    async fn test_update_comment_by_author() {
        let pool = setup_test_db().await;
        let (task_id, user_id) = setup_task_and_user(&pool).await;

        let comment = create_comment(
            &pool,
            task_id,
            user_id,
            &CreateCommentRequest {
                content: "Original".to_string(),
            },
        )
        .await
        .unwrap();

        let updated = update_comment(
            &pool,
            comment.id,
            user_id,
            &UpdateCommentRequest {
                content: "Updated".to_string(),
            },
        )
        .await
        .unwrap();

        assert_eq!(updated.content, "Updated");
    }

    #[tokio::test]
    async fn test_update_comment_by_non_author_fails() {
        let pool = setup_test_db().await;
        let (task_id, user_id) = setup_task_and_user(&pool).await;

        let comment = create_comment(
            &pool,
            task_id,
            user_id,
            &CreateCommentRequest {
                content: "Original".to_string(),
            },
        )
        .await
        .unwrap();

        let other_user = user_service::create_user(
            &pool,
            &RegisterRequest {
                email: "bob@test.com".to_string(),
                name: "Bob".to_string(),
                password: "password123".to_string(),
            },
        )
        .await
        .unwrap();

        let result = update_comment(
            &pool,
            comment.id,
            other_user.id,
            &UpdateCommentRequest {
                content: "Hacked".to_string(),
            },
        )
        .await;

        assert!(matches!(result, Err(AppError::Forbidden(_))));
    }

    #[tokio::test]
    async fn test_delete_comment_by_author() {
        let pool = setup_test_db().await;
        let (task_id, user_id) = setup_task_and_user(&pool).await;

        let comment = create_comment(
            &pool,
            task_id,
            user_id,
            &CreateCommentRequest {
                content: "To delete".to_string(),
            },
        )
        .await
        .unwrap();

        delete_comment(&pool, comment.id, user_id).await.unwrap();

        let result = get_comment(&pool, comment.id).await;
        assert!(matches!(result, Err(AppError::NotFound(_))));
    }

    #[tokio::test]
    async fn test_get_comment_not_found() {
        let pool = setup_test_db().await;

        let result = get_comment(&pool, Uuid::new_v4()).await;
        assert!(matches!(result, Err(AppError::NotFound(_))));
    }
}
