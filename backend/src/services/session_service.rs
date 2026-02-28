use chrono::{Duration, Utc};
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::error::{AppError, AppResult};
use crate::models::user::Session;
use crate::repositories::session_repository;

/// Create a new session for a user
pub async fn create_session(
    pool: &SqlitePool,
    user_id: Uuid,
    duration_hours: u64,
) -> AppResult<Session> {
    let id = Uuid::new_v4();
    let now = Utc::now();
    let expires_at = now + Duration::hours(duration_hours as i64);

    session_repository::insert(pool, id, user_id, now, expires_at).await?;

    Ok(Session {
        id: id.to_string(),
        user_id: user_id.to_string(),
        created_at: now,
        expires_at,
        last_active_at: now,
    })
}

/// Get session by ID, returns None if not found or expired
pub async fn get_session(pool: &SqlitePool, session_id: Uuid) -> AppResult<Option<Session>> {
    let now = Utc::now();
    session_repository::find_by_id_not_expired(pool, session_id, now).await
}

/// Validate and update session's last_active_at timestamp
/// Returns the session if valid, Unauthorized error if not found or expired
#[tracing::instrument(skip(pool))]
pub async fn validate_session(pool: &SqlitePool, session_id: Uuid) -> AppResult<Session> {
    let session = get_session(pool, session_id).await?;

    match session {
        Some(sess) => {
            let now = Utc::now();
            session_repository::update_last_active_at(pool, session_id, now).await?;

            Ok(Session {
                last_active_at: now,
                ..sess
            })
        }
        None => Err(AppError::Unauthorized),
    }
}

/// Delete a session (logout)
pub async fn delete_session(pool: &SqlitePool, session_id: Uuid) -> AppResult<()> {
    session_repository::delete_by_id(pool, session_id).await
}

/// Delete all sessions for a user
pub async fn delete_user_sessions(pool: &SqlitePool, user_id: Uuid) -> AppResult<()> {
    session_repository::delete_by_user_id(pool, user_id).await
}

/// Delete all sessions for a user and create a new one atomically.
/// Prevents session fixation by ensuring no window between delete and create.
#[tracing::instrument(skip(pool))]
pub async fn rotate_session(
    pool: &SqlitePool,
    user_id: Uuid,
    duration_hours: u64,
) -> AppResult<Session> {
    let mut tx = pool.begin().await?;

    session_repository::delete_by_user_id_tx(&mut *tx, user_id).await?;

    let id = Uuid::new_v4();
    let now = Utc::now();
    let expires_at = now + Duration::hours(duration_hours as i64);

    session_repository::insert_tx(&mut *tx, id, user_id, now, expires_at).await?;

    tx.commit().await?;

    Ok(Session {
        id: id.to_string(),
        user_id: user_id.to_string(),
        created_at: now,
        expires_at,
        last_active_at: now,
    })
}

/// Clean up expired sessions
pub async fn cleanup_expired_sessions(pool: &SqlitePool) -> AppResult<u64> {
    let now = Utc::now();
    session_repository::delete_expired(pool, now).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::user::RegisterRequest;
    use crate::services::user_service;
    use crate::test_helpers::setup_test_db;

    async fn create_test_user(pool: &SqlitePool) -> Uuid {
        let req = RegisterRequest {
            email: format!("test-{}@example.com", Uuid::new_v4()),
            name: "Test User".to_string(),
            password: "correct horse battery staple purple".to_string(),
        };
        let user = user_service::create_user(pool, &req)
            .await
            .expect("Failed to create user");
        user.id
    }

    #[tokio::test]
    async fn test_create_session_returns_valid_session() {
        let pool = setup_test_db().await;
        let user_id = create_test_user(&pool).await;

        let session = create_session(&pool, user_id, 24)
            .await
            .expect("Failed to create session");

        assert_eq!(session.user_id, user_id.to_string());
        assert!(session.expires_at > session.created_at);
    }

    #[tokio::test]
    async fn test_get_session_returns_existing() {
        let pool = setup_test_db().await;
        let user_id = create_test_user(&pool).await;
        let created = create_session(&pool, user_id, 24)
            .await
            .expect("Failed to create");

        let session_id: Uuid = created.id.parse().unwrap();
        let found = get_session(&pool, session_id)
            .await
            .expect("Failed to get")
            .expect("Session should exist");

        assert_eq!(found.id, created.id);
    }

    #[tokio::test]
    async fn test_get_session_returns_none_for_nonexistent() {
        let pool = setup_test_db().await;
        let random_id = Uuid::new_v4();

        let result = get_session(&pool, random_id)
            .await
            .expect("Should not error");

        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_validate_session_updates_last_active() {
        let pool = setup_test_db().await;
        let user_id = create_test_user(&pool).await;
        let created = create_session(&pool, user_id, 24)
            .await
            .expect("Failed to create");

        // Wait a tiny bit to ensure timestamp difference
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        let session_id: Uuid = created.id.parse().unwrap();
        let validated = validate_session(&pool, session_id)
            .await
            .expect("Failed to validate");

        assert!(validated.last_active_at >= created.last_active_at);
    }

    #[tokio::test]
    async fn test_validate_session_returns_unauthorized_for_nonexistent() {
        let pool = setup_test_db().await;
        let random_id = Uuid::new_v4();

        let result = validate_session(&pool, random_id).await;

        assert!(matches!(result, Err(AppError::Unauthorized)));
    }

    #[tokio::test]
    async fn test_delete_session_removes_from_db() {
        let pool = setup_test_db().await;
        let user_id = create_test_user(&pool).await;
        let session = create_session(&pool, user_id, 24)
            .await
            .expect("Failed to create");

        let session_id: Uuid = session.id.parse().unwrap();
        delete_session(&pool, session_id)
            .await
            .expect("Failed to delete");

        let found = get_session(&pool, session_id)
            .await
            .expect("Should not error");
        assert!(found.is_none());
    }

    #[tokio::test]
    async fn test_delete_user_sessions_removes_all() {
        let pool = setup_test_db().await;
        let user_id = create_test_user(&pool).await;

        // Create multiple sessions
        let session1 = create_session(&pool, user_id, 24).await.unwrap();
        let session2 = create_session(&pool, user_id, 24).await.unwrap();

        delete_user_sessions(&pool, user_id)
            .await
            .expect("Failed to delete");

        let session1_id: Uuid = session1.id.parse().unwrap();
        let session2_id: Uuid = session2.id.parse().unwrap();

        assert!(get_session(&pool, session1_id).await.unwrap().is_none());
        assert!(get_session(&pool, session2_id).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_rotate_session_deletes_old_and_creates_new() {
        let pool = setup_test_db().await;
        let user_id = create_test_user(&pool).await;

        let old1 = create_session(&pool, user_id, 24).await.unwrap();
        let old2 = create_session(&pool, user_id, 24).await.unwrap();

        let new_session = rotate_session(&pool, user_id, 24).await.unwrap();

        // Old sessions should be gone
        let old1_id: Uuid = old1.id.parse().unwrap();
        let old2_id: Uuid = old2.id.parse().unwrap();
        assert!(get_session(&pool, old1_id).await.unwrap().is_none());
        assert!(get_session(&pool, old2_id).await.unwrap().is_none());

        // New session should exist
        let new_id: Uuid = new_session.id.parse().unwrap();
        assert!(get_session(&pool, new_id).await.unwrap().is_some());
    }

    #[tokio::test]
    async fn test_cleanup_expired_sessions() {
        let pool = setup_test_db().await;
        let user_id = create_test_user(&pool).await;

        // Create a session with 0 hours duration (expires immediately)
        let id = Uuid::new_v4();
        let now = Utc::now();
        let expired_at = now - Duration::hours(1); // Already expired

        sqlx::query(
            r#"
            INSERT INTO sessions (id, user_id, created_at, expires_at, last_active_at)
            VALUES ($1, $2, $3, $4, $5)
            "#,
        )
        .bind(id.to_string())
        .bind(user_id.to_string())
        .bind(now)
        .bind(expired_at)
        .bind(now)
        .execute(&pool)
        .await
        .unwrap();

        let deleted = cleanup_expired_sessions(&pool)
            .await
            .expect("Failed to cleanup");

        assert_eq!(deleted, 1);
        assert!(get_session(&pool, id).await.unwrap().is_none());
    }
}
