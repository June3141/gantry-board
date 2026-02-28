use chrono::{DateTime, Utc};
use sqlx::sqlite::SqliteConnection;
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::error::AppResult;
use crate::models::user::Session;

pub async fn insert(
    pool: &SqlitePool,
    id: Uuid,
    user_id: Uuid,
    now: DateTime<Utc>,
    expires_at: DateTime<Utc>,
) -> AppResult<()> {
    sqlx::query(
        r#"
        INSERT INTO sessions (id, user_id, created_at, expires_at, last_active_at)
        VALUES ($1, $2, $3, $4, $5)
        "#,
    )
    .bind(id.to_string())
    .bind(user_id.to_string())
    .bind(now)
    .bind(expires_at)
    .bind(now)
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn insert_tx(
    conn: &mut SqliteConnection,
    id: Uuid,
    user_id: Uuid,
    now: DateTime<Utc>,
    expires_at: DateTime<Utc>,
) -> AppResult<()> {
    sqlx::query(
        r#"
        INSERT INTO sessions (id, user_id, created_at, expires_at, last_active_at)
        VALUES ($1, $2, $3, $4, $5)
        "#,
    )
    .bind(id.to_string())
    .bind(user_id.to_string())
    .bind(now)
    .bind(expires_at)
    .bind(now)
    .execute(&mut *conn)
    .await?;

    Ok(())
}

pub async fn find_by_id_not_expired(
    pool: &SqlitePool,
    session_id: Uuid,
    now: DateTime<Utc>,
) -> AppResult<Option<Session>> {
    let session = sqlx::query_as::<_, Session>(
        r#"
        SELECT id, user_id, created_at, expires_at, last_active_at
        FROM sessions
        WHERE id = $1 AND expires_at > $2
        "#,
    )
    .bind(session_id.to_string())
    .bind(now)
    .fetch_optional(pool)
    .await?;

    Ok(session)
}

pub async fn update_last_active_at(
    pool: &SqlitePool,
    session_id: Uuid,
    now: DateTime<Utc>,
) -> AppResult<()> {
    sqlx::query(
        r#"
        UPDATE sessions
        SET last_active_at = $1
        WHERE id = $2
        "#,
    )
    .bind(now)
    .bind(session_id.to_string())
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn delete_by_id(pool: &SqlitePool, session_id: Uuid) -> AppResult<()> {
    sqlx::query("DELETE FROM sessions WHERE id = $1")
        .bind(session_id.to_string())
        .execute(pool)
        .await?;

    Ok(())
}

pub async fn delete_by_user_id(pool: &SqlitePool, user_id: Uuid) -> AppResult<()> {
    sqlx::query("DELETE FROM sessions WHERE user_id = $1")
        .bind(user_id.to_string())
        .execute(pool)
        .await?;

    Ok(())
}

pub async fn delete_by_user_id_tx(conn: &mut SqliteConnection, user_id: Uuid) -> AppResult<()> {
    sqlx::query("DELETE FROM sessions WHERE user_id = $1")
        .bind(user_id.to_string())
        .execute(&mut *conn)
        .await?;

    Ok(())
}

pub async fn delete_expired(pool: &SqlitePool, now: DateTime<Utc>) -> AppResult<u64> {
    let result = sqlx::query("DELETE FROM sessions WHERE expires_at <= $1")
        .bind(now)
        .execute(pool)
        .await?;

    Ok(result.rows_affected())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::user::RegisterRequest;
    use crate::services::user_service;
    use crate::test_helpers::setup_test_db;
    use chrono::Duration;

    async fn create_test_user(pool: &SqlitePool) -> Uuid {
        let req = RegisterRequest {
            email: format!("test-{}@example.com", Uuid::new_v4()),
            name: "Test User".to_string(),
            password: "correct horse battery staple purple".to_string(),
        };
        let user = user_service::create_user(pool, &req)
            .await
            .expect("create user");
        user.id
    }

    #[tokio::test]
    async fn test_insert_and_find() {
        let pool = setup_test_db().await;
        let user_id = create_test_user(&pool).await;

        let id = Uuid::new_v4();
        let now = Utc::now();
        let expires_at = now + Duration::hours(24);

        insert(&pool, id, user_id, now, expires_at)
            .await
            .expect("insert");

        let session = find_by_id_not_expired(&pool, id, now)
            .await
            .expect("find")
            .expect("should exist");

        assert_eq!(session.id, id.to_string());
        assert_eq!(session.user_id, user_id.to_string());
    }

    #[tokio::test]
    async fn test_find_returns_none_for_expired() {
        let pool = setup_test_db().await;
        let user_id = create_test_user(&pool).await;

        let id = Uuid::new_v4();
        let now = Utc::now();
        let expires_at = now - Duration::hours(1);

        insert(&pool, id, user_id, now, expires_at)
            .await
            .expect("insert");

        let result = find_by_id_not_expired(&pool, id, now).await.expect("find");
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_find_returns_none_for_nonexistent() {
        let pool = setup_test_db().await;
        let result = find_by_id_not_expired(&pool, Uuid::new_v4(), Utc::now())
            .await
            .expect("find");
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_update_last_active_at() {
        let pool = setup_test_db().await;
        let user_id = create_test_user(&pool).await;

        let id = Uuid::new_v4();
        let now = Utc::now();
        let expires_at = now + Duration::hours(24);

        insert(&pool, id, user_id, now, expires_at)
            .await
            .expect("insert");

        let later = now + Duration::seconds(10);
        update_last_active_at(&pool, id, later)
            .await
            .expect("update");

        let session = find_by_id_not_expired(&pool, id, now)
            .await
            .expect("find")
            .expect("should exist");
        assert!(session.last_active_at >= now);
    }

    #[tokio::test]
    async fn test_delete_by_id() {
        let pool = setup_test_db().await;
        let user_id = create_test_user(&pool).await;

        let id = Uuid::new_v4();
        let now = Utc::now();
        let expires_at = now + Duration::hours(24);

        insert(&pool, id, user_id, now, expires_at)
            .await
            .expect("insert");

        delete_by_id(&pool, id).await.expect("delete");

        let result = find_by_id_not_expired(&pool, id, now).await.expect("find");
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_delete_by_user_id() {
        let pool = setup_test_db().await;
        let user_id = create_test_user(&pool).await;

        let now = Utc::now();
        let expires_at = now + Duration::hours(24);

        let id1 = Uuid::new_v4();
        let id2 = Uuid::new_v4();
        insert(&pool, id1, user_id, now, expires_at)
            .await
            .expect("insert 1");
        insert(&pool, id2, user_id, now, expires_at)
            .await
            .expect("insert 2");

        delete_by_user_id(&pool, user_id).await.expect("delete");

        assert!(find_by_id_not_expired(&pool, id1, now)
            .await
            .unwrap()
            .is_none());
        assert!(find_by_id_not_expired(&pool, id2, now)
            .await
            .unwrap()
            .is_none());
    }

    #[tokio::test]
    async fn test_insert_tx_and_delete_by_user_id_tx() {
        let pool = setup_test_db().await;
        let user_id = create_test_user(&pool).await;

        let now = Utc::now();
        let expires_at = now + Duration::hours(24);

        // Insert via transaction
        let id = Uuid::new_v4();
        let mut tx = pool.begin().await.unwrap();
        insert_tx(&mut *tx, id, user_id, now, expires_at)
            .await
            .expect("insert_tx");
        tx.commit().await.unwrap();

        let session = find_by_id_not_expired(&pool, id, now)
            .await
            .unwrap()
            .expect("should exist");
        assert_eq!(session.id, id.to_string());

        // Delete via transaction
        let mut tx = pool.begin().await.unwrap();
        delete_by_user_id_tx(&mut *tx, user_id)
            .await
            .expect("delete_tx");
        tx.commit().await.unwrap();

        assert!(find_by_id_not_expired(&pool, id, now)
            .await
            .unwrap()
            .is_none());
    }

    #[tokio::test]
    async fn test_delete_expired() {
        let pool = setup_test_db().await;
        let user_id = create_test_user(&pool).await;

        let now = Utc::now();
        // Insert an already-expired session
        let id = Uuid::new_v4();
        let past = now - Duration::hours(2);
        let expired_at = now - Duration::hours(1);
        insert(&pool, id, user_id, past, expired_at)
            .await
            .expect("insert expired");

        // Insert a valid session
        let valid_id = Uuid::new_v4();
        insert(&pool, valid_id, user_id, now, now + Duration::hours(24))
            .await
            .expect("insert valid");

        let deleted = delete_expired(&pool, now).await.expect("delete_expired");
        assert_eq!(deleted, 1);

        // Valid session should still exist
        assert!(find_by_id_not_expired(&pool, valid_id, now)
            .await
            .unwrap()
            .is_some());
    }
}
