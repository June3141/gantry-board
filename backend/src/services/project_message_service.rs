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
    async fn test_create_message() {
        let pool = setup_test_db().await;
        let (project_id, user_id) = setup_project_and_user(&pool).await;

        let req = CreateMessageRequest {
            content: "Hello!".to_string(),
        };
        let msg = create_message(&pool, project_id, user_id, &req)
            .await
            .unwrap();

        assert_eq!(msg.project_id, project_id);
        assert_eq!(msg.user_id, user_id);
        assert_eq!(msg.content, "Hello!");
        assert_eq!(msg.user_name, "MsgUser");
    }

    #[tokio::test]
    async fn test_get_message_not_found() {
        let pool = setup_test_db().await;

        let result = get_message(&pool, Uuid::new_v4()).await;
        assert!(matches!(result, Err(AppError::NotFound(_))));
    }

    #[tokio::test]
    async fn test_list_messages_returns_desc_order() {
        let pool = setup_test_db().await;
        let (project_id, user_id) = setup_project_and_user(&pool).await;

        create_message(
            &pool,
            project_id,
            user_id,
            &CreateMessageRequest {
                content: "First".to_string(),
            },
        )
        .await
        .unwrap();

        create_message(
            &pool,
            project_id,
            user_id,
            &CreateMessageRequest {
                content: "Second".to_string(),
            },
        )
        .await
        .unwrap();

        let msgs = list_messages(&pool, project_id, None, 50).await.unwrap();
        assert_eq!(msgs.len(), 2);
        // DESC order: newest first
        assert_eq!(msgs[0].content, "Second");
        assert_eq!(msgs[1].content, "First");
    }

    #[tokio::test]
    async fn test_list_messages_respects_limit() {
        let pool = setup_test_db().await;
        let (project_id, user_id) = setup_project_and_user(&pool).await;

        for i in 0..5 {
            create_message(
                &pool,
                project_id,
                user_id,
                &CreateMessageRequest {
                    content: format!("Msg {i}"),
                },
            )
            .await
            .unwrap();
        }

        let msgs = list_messages(&pool, project_id, None, 2).await.unwrap();
        assert_eq!(msgs.len(), 2);
    }

    #[tokio::test]
    async fn test_delete_message_by_author() {
        let pool = setup_test_db().await;
        let (project_id, user_id) = setup_project_and_user(&pool).await;

        let msg = create_message(
            &pool,
            project_id,
            user_id,
            &CreateMessageRequest {
                content: "Delete me".to_string(),
            },
        )
        .await
        .unwrap();

        delete_message(&pool, msg.id, user_id).await.unwrap();

        let result = get_message(&pool, msg.id).await;
        assert!(matches!(result, Err(AppError::NotFound(_))));
    }

    #[tokio::test]
    async fn test_delete_message_by_non_author_fails() {
        let pool = setup_test_db().await;
        let (project_id, user_id) = setup_project_and_user(&pool).await;

        let msg = create_message(
            &pool,
            project_id,
            user_id,
            &CreateMessageRequest {
                content: "Can't touch this".to_string(),
            },
        )
        .await
        .unwrap();

        let other = user_service::create_user(
            &pool,
            &RegisterRequest {
                email: "other@test.com".to_string(),
                name: "Other".to_string(),
                password: "password123".to_string(),
            },
        )
        .await
        .unwrap();

        let result = delete_message(&pool, msg.id, other.id).await;
        assert!(matches!(result, Err(AppError::Forbidden(_))));
    }

    #[tokio::test]
    async fn test_delete_message_admin_succeeds() {
        let pool = setup_test_db().await;
        let (project_id, user_id) = setup_project_and_user(&pool).await;

        let msg = create_message(
            &pool,
            project_id,
            user_id,
            &CreateMessageRequest {
                content: "Admin delete".to_string(),
            },
        )
        .await
        .unwrap();

        delete_message_admin(&pool, msg.id).await.unwrap();
        assert!(matches!(
            get_message(&pool, msg.id).await,
            Err(AppError::NotFound(_))
        ));
    }

    #[tokio::test]
    async fn test_delete_message_admin_not_found() {
        let pool = setup_test_db().await;

        let result = delete_message_admin(&pool, Uuid::new_v4()).await;
        assert!(matches!(result, Err(AppError::NotFound(_))));
    }
}
