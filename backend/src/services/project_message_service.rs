use chrono::{DateTime, Utc};
use sqlx::{FromRow, SqlitePool};
use uuid::Uuid;

use crate::error::{AppError, AppResult};
use crate::models::project_message::{CreateMessageRequest, ProjectMessage};

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
    type Error = AppError;

    fn try_from(row: MessageRow) -> Result<Self, Self::Error> {
        Ok(Self {
            id: row
                .id
                .parse()
                .map_err(|e: uuid::Error| AppError::Internal(e.to_string()))?,
            project_id: row
                .project_id
                .parse()
                .map_err(|e: uuid::Error| AppError::Internal(e.to_string()))?,
            user_id: row
                .user_id
                .parse()
                .map_err(|e: uuid::Error| AppError::Internal(e.to_string()))?,
            user_name: row.user_name,
            content: row.content,
            created_at: row.created_at,
        })
    }
}

pub async fn create_message(
    pool: &SqlitePool,
    project_id: Uuid,
    user_id: Uuid,
    req: &CreateMessageRequest,
) -> AppResult<ProjectMessage> {
    let id = Uuid::new_v4();
    sqlx::query(
        r#"
        INSERT INTO project_messages (id, project_id, user_id, content)
        VALUES ($1, $2, $3, $4)
        "#,
    )
    .bind(id.to_string())
    .bind(project_id.to_string())
    .bind(user_id.to_string())
    .bind(&req.content)
    .execute(pool)
    .await?;

    get_message(pool, id).await
}

pub async fn get_message(pool: &SqlitePool, message_id: Uuid) -> AppResult<ProjectMessage> {
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

    row.ok_or_else(|| AppError::NotFound(format!("project message not found: {message_id}")))
        .and_then(ProjectMessage::try_from)
}

pub async fn list_messages(
    pool: &SqlitePool,
    project_id: Uuid,
    before: Option<DateTime<Utc>>,
    limit: i64,
) -> AppResult<Vec<ProjectMessage>> {
    let rows = match before {
        Some(cursor) => {
            sqlx::query_as::<_, MessageRow>(
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
            .await?
        }
        None => {
            sqlx::query_as::<_, MessageRow>(
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
            .await?
        }
    };

    rows.into_iter().map(ProjectMessage::try_from).collect()
}

pub async fn delete_message(pool: &SqlitePool, message_id: Uuid, user_id: Uuid) -> AppResult<()> {
    let existing = get_message(pool, message_id).await?;
    if existing.user_id != user_id {
        return Err(AppError::Forbidden(
            "only the message author can delete this message".to_string(),
        ));
    }
    sqlx::query("DELETE FROM project_messages WHERE id = $1")
        .bind(message_id.to_string())
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn delete_message_admin(pool: &SqlitePool, message_id: Uuid) -> AppResult<()> {
    let result = sqlx::query("DELETE FROM project_messages WHERE id = $1")
        .bind(message_id.to_string())
        .execute(pool)
        .await?;
    if result.rows_affected() == 0 {
        return Err(AppError::NotFound(format!(
            "project message not found: {message_id}"
        )));
    }
    Ok(())
}
