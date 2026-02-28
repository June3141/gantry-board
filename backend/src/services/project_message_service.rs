use chrono::{DateTime, Utc};
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::error::{AppError, AppResult};
use crate::models::project_message::{CreateMessageRequest, ProjectMessage};
use crate::repositories::project_message_repository;

pub async fn create_message(
    pool: &SqlitePool,
    project_id: Uuid,
    user_id: Uuid,
    req: &CreateMessageRequest,
) -> AppResult<ProjectMessage> {
    let id = Uuid::new_v4();
    project_message_repository::insert(pool, id, project_id, user_id, &req.content).await?;
    project_message_repository::find_by_id(pool, id).await
}

pub async fn get_message(pool: &SqlitePool, message_id: Uuid) -> AppResult<ProjectMessage> {
    project_message_repository::find_by_id(pool, message_id).await
}

pub async fn list_messages(
    pool: &SqlitePool,
    project_id: Uuid,
    before: Option<DateTime<Utc>>,
    limit: i64,
) -> AppResult<Vec<ProjectMessage>> {
    match before {
        Some(cursor) => {
            project_message_repository::find_by_project_before_cursor(
                pool, project_id, cursor, limit,
            )
            .await
        }
        None => project_message_repository::find_by_project(pool, project_id, limit).await,
    }
}

pub async fn delete_message(pool: &SqlitePool, message_id: Uuid, user_id: Uuid) -> AppResult<()> {
    let existing = project_message_repository::find_by_id(pool, message_id).await?;
    if existing.user_id != user_id {
        return Err(AppError::Forbidden(
            "only the message author can delete this message".to_string(),
        ));
    }
    project_message_repository::delete(pool, message_id).await?;
    Ok(())
}

pub async fn delete_message_admin(pool: &SqlitePool, message_id: Uuid) -> AppResult<()> {
    let rows_affected = project_message_repository::delete(pool, message_id).await?;
    if rows_affected == 0 {
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
