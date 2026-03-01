use chrono::{DateTime, Utc};
use sqlx::{FromRow, SqlitePool};
use uuid::Uuid;

use crate::error::{AppError, AppResult};
use crate::models::project_message::ProjectMessage;

#[derive(FromRow)]
struct MessageRow {
    id: String,
    project_id: String,
    user_id: String,
    user_name: String,
    content: String,
    created_at: DateTime<Utc>,
}

impl TryFrom<MessageRow> for ProjectMessage {
    type Error = uuid::Error;

    fn try_from(row: MessageRow) -> Result<Self, Self::Error> {
        Ok(ProjectMessage {
            id: row.id.parse()?,
            project_id: row.project_id.parse()?,
            user_id: row.user_id.parse()?,
            user_name: row.user_name,
            content: row.content,
            created_at: row.created_at,
        })
    }
}

fn row_to_message(row: MessageRow) -> AppResult<ProjectMessage> {
    row.try_into()
        .map_err(|e: uuid::Error| AppError::Internal(e.to_string()))
}

/// Insert a new project message.
pub async fn insert(
    pool: &SqlitePool,
    id: Uuid,
    project_id: Uuid,
    user_id: Uuid,
    content: &str,
) -> AppResult<()> {
    sqlx::query(
        r#"
        INSERT INTO project_messages (id, project_id, user_id, content)
        VALUES ($1, $2, $3, $4)
        "#,
    )
    .bind(id.to_string())
    .bind(project_id.to_string())
    .bind(user_id.to_string())
    .bind(content)
    .execute(pool)
    .await?;

    Ok(())
}

/// Find a message by its ID (with user name via JOIN).
pub async fn find_by_id(pool: &SqlitePool, message_id: Uuid) -> AppResult<ProjectMessage> {
    let row = sqlx::query_as::<_, MessageRow>(
        r#"
        SELECT m.id, m.project_id, m.user_id, u.name AS user_name, m.content, m.created_at
        FROM project_messages m
        JOIN users u ON m.user_id = u.id
        WHERE m.id = $1
        "#,
    )
    .bind(message_id.to_string())
    .fetch_optional(pool)
    .await?;

    match row {
        Some(r) => row_to_message(r),
        None => Err(AppError::NotFound(format!(
            "project message not found: {}",
            message_id
        ))),
    }
}

/// Find messages by project with a cursor (created_at < cursor), ordered DESC, limited.
pub async fn find_by_project_before_cursor(
    pool: &SqlitePool,
    project_id: Uuid,
    cursor: DateTime<Utc>,
    limit: i64,
) -> AppResult<Vec<ProjectMessage>> {
    let rows = sqlx::query_as::<_, MessageRow>(
        r#"
        SELECT m.id, m.project_id, m.user_id, u.name AS user_name, m.content, m.created_at
        FROM project_messages m
        JOIN users u ON m.user_id = u.id
        WHERE m.project_id = $1 AND m.created_at < $2
        ORDER BY m.created_at DESC
        LIMIT $3
        "#,
    )
    .bind(project_id.to_string())
    .bind(cursor)
    .bind(limit)
    .fetch_all(pool)
    .await?;

    rows.into_iter().map(row_to_message).collect()
}

/// Find messages by project (no cursor), ordered DESC, limited.
pub async fn find_by_project(
    pool: &SqlitePool,
    project_id: Uuid,
    limit: i64,
) -> AppResult<Vec<ProjectMessage>> {
    let rows = sqlx::query_as::<_, MessageRow>(
        r#"
        SELECT m.id, m.project_id, m.user_id, u.name AS user_name, m.content, m.created_at
        FROM project_messages m
        JOIN users u ON m.user_id = u.id
        WHERE m.project_id = $1
        ORDER BY m.created_at DESC
        LIMIT $2
        "#,
    )
    .bind(project_id.to_string())
    .bind(limit)
    .fetch_all(pool)
    .await?;

    rows.into_iter().map(row_to_message).collect()
}

/// Delete a message. Returns the number of rows affected.
pub async fn delete(pool: &SqlitePool, message_id: Uuid) -> AppResult<u64> {
    let result = sqlx::query("DELETE FROM project_messages WHERE id = $1")
        .bind(message_id.to_string())
        .execute(pool)
        .await?;

    Ok(result.rows_affected())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::project::CreateProjectRequest;
    use crate::models::user::RegisterRequest;
    use crate::services::{project_service, user_service};
    use crate::test_helpers::setup_test_db;

    async fn setup_project_and_user(pool: &SqlitePool) -> (Uuid, Uuid) {
        let user = user_service::create_user(
            pool,
            &RegisterRequest {
                email: "msg_user@test.com".to_string(),
                name: "MsgUser".to_string(),
                password: "password123".to_string(),
            },
        )
        .await
        .unwrap();

        let project = project_service::create_project(
            pool,
            &CreateProjectRequest {
                name: "MsgProject".to_string(),
                description: None,
                repository_path: None,
            },
        )
        .await
        .unwrap();

        (project.id, user.id)
    }

    #[tokio::test]
    async fn test_insert_and_find_by_id() {
        let pool = setup_test_db().await;
        let (project_id, user_id) = setup_project_and_user(&pool).await;

        let id = Uuid::new_v4();
        insert(&pool, id, project_id, user_id, "Hello!")
            .await
            .unwrap();

        let msg = find_by_id(&pool, id).await.unwrap();
        assert_eq!(msg.id, id);
        assert_eq!(msg.project_id, project_id);
        assert_eq!(msg.user_id, user_id);
        assert_eq!(msg.content, "Hello!");
        assert_eq!(msg.user_name, "MsgUser");
    }

    #[tokio::test]
    async fn test_find_by_id_not_found() {
        let pool = setup_test_db().await;
        let result = find_by_id(&pool, Uuid::new_v4()).await;
        assert!(matches!(result, Err(AppError::NotFound(_))));
    }

    #[tokio::test]
    async fn test_find_by_project_returns_desc_order() {
        let pool = setup_test_db().await;
        let (project_id, user_id) = setup_project_and_user(&pool).await;

        insert(&pool, Uuid::new_v4(), project_id, user_id, "First")
            .await
            .unwrap();
        insert(&pool, Uuid::new_v4(), project_id, user_id, "Second")
            .await
            .unwrap();

        let msgs = find_by_project(&pool, project_id, 50).await.unwrap();
        assert_eq!(msgs.len(), 2);
        // DESC order: newest first
        assert_eq!(msgs[0].content, "Second");
        assert_eq!(msgs[1].content, "First");
    }

    #[tokio::test]
    async fn test_find_by_project_respects_limit() {
        let pool = setup_test_db().await;
        let (project_id, user_id) = setup_project_and_user(&pool).await;

        for i in 0..5 {
            insert(
                &pool,
                Uuid::new_v4(),
                project_id,
                user_id,
                &format!("Msg {i}"),
            )
            .await
            .unwrap();
        }

        let msgs = find_by_project(&pool, project_id, 2).await.unwrap();
        assert_eq!(msgs.len(), 2);
    }

    #[tokio::test]
    async fn test_find_by_project_before_cursor() {
        let pool = setup_test_db().await;
        let (project_id, user_id) = setup_project_and_user(&pool).await;

        // Insert with explicit different timestamps to ensure cursor works reliably.
        let old_id = Uuid::new_v4();
        let new_id = Uuid::new_v4();
        let old_time = "2025-01-01T00:00:00.000Z";
        let new_time = "2025-06-01T00:00:00.000Z";

        sqlx::query(
            "INSERT INTO project_messages (id, project_id, user_id, content, created_at) VALUES ($1, $2, $3, $4, $5)",
        )
        .bind(old_id.to_string())
        .bind(project_id.to_string())
        .bind(user_id.to_string())
        .bind("Old")
        .bind(old_time)
        .execute(&pool)
        .await
        .unwrap();

        sqlx::query(
            "INSERT INTO project_messages (id, project_id, user_id, content, created_at) VALUES ($1, $2, $3, $4, $5)",
        )
        .bind(new_id.to_string())
        .bind(project_id.to_string())
        .bind(user_id.to_string())
        .bind("New")
        .bind(new_time)
        .execute(&pool)
        .await
        .unwrap();

        // Get all messages to determine cursor
        let all = find_by_project(&pool, project_id, 50).await.unwrap();
        assert_eq!(all.len(), 2);
        // DESC order: New first
        assert_eq!(all[0].content, "New");

        // Use the newest message's created_at as cursor -> should return only the older one
        let cursor = all[0].created_at;
        let before = find_by_project_before_cursor(&pool, project_id, cursor, 50)
            .await
            .unwrap();
        assert_eq!(before.len(), 1);
        assert_eq!(before[0].content, "Old");
    }

    #[tokio::test]
    async fn test_find_by_project_empty() {
        let pool = setup_test_db().await;
        let msgs = find_by_project(&pool, Uuid::new_v4(), 50).await.unwrap();
        assert!(msgs.is_empty());
    }

    #[tokio::test]
    async fn test_delete() {
        let pool = setup_test_db().await;
        let (project_id, user_id) = setup_project_and_user(&pool).await;

        let id = Uuid::new_v4();
        insert(&pool, id, project_id, user_id, "Delete me")
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
