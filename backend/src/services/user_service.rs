use chrono::Utc;
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::auth::password::{hash_password, verify_password};
use crate::error::{AppError, AppResult};
use crate::models::user::{RegisterRequest, User, UserWithPassword};

/// Create a new user with hashed password
pub async fn create_user(pool: &SqlitePool, req: &RegisterRequest) -> AppResult<User> {
    let id = Uuid::new_v4();
    let now = Utc::now();
    let password_hash = hash_password(&req.password)?;

    sqlx::query(
        r#"
        INSERT INTO users (id, email, name, password_hash, created_at, updated_at)
        VALUES ($1, $2, $3, $4, $5, $6)
        "#,
    )
    .bind(id.to_string())
    .bind(&req.email)
    .bind(&req.name)
    .bind(&password_hash)
    .bind(now)
    .bind(now)
    .execute(pool)
    .await?;

    Ok(User {
        id,
        name: req.name.clone(),
        email: req.email.clone(),
        created_at: now,
        updated_at: now,
    })
}

/// Get user by ID
pub async fn get_user(pool: &SqlitePool, id: Uuid) -> AppResult<User> {
    let row = sqlx::query_as::<_, UserWithPassword>(
        r#"
        SELECT id, email, name, password_hash, created_at, updated_at
        FROM users
        WHERE id = $1
        "#,
    )
    .bind(id.to_string())
    .fetch_optional(pool)
    .await?;

    row.map(|r| r.try_into())
        .transpose()
        .map_err(|e: uuid::Error| AppError::Internal(e.to_string()))?
        .ok_or_else(|| AppError::NotFound(format!("user {} not found", id)))
}

/// Get user by email
pub async fn get_user_by_email(pool: &SqlitePool, email: &str) -> AppResult<User> {
    let row = sqlx::query_as::<_, UserWithPassword>(
        r#"
        SELECT id, email, name, password_hash, created_at, updated_at
        FROM users
        WHERE email = $1
        "#,
    )
    .bind(email)
    .fetch_optional(pool)
    .await?;

    row.map(|r| r.try_into())
        .transpose()
        .map_err(|e: uuid::Error| AppError::Internal(e.to_string()))?
        .ok_or_else(|| AppError::NotFound(format!("user with email {} not found", email)))
}

/// Authenticate user by email and password
/// Returns user if credentials are valid, InvalidCredentials error otherwise
pub async fn authenticate_user(pool: &SqlitePool, email: &str, password: &str) -> AppResult<User> {
    let row = sqlx::query_as::<_, UserWithPassword>(
        r#"
        SELECT id, email, name, password_hash, created_at, updated_at
        FROM users
        WHERE email = $1
        "#,
    )
    .bind(email)
    .fetch_optional(pool)
    .await?;

    let user_with_password = row.ok_or(AppError::InvalidCredentials)?;

    if !verify_password(password, &user_with_password.password_hash)? {
        return Err(AppError::InvalidCredentials);
    }

    user_with_password
        .try_into()
        .map_err(|e: uuid::Error| AppError::Internal(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers::setup_test_db;

    fn test_register_request() -> RegisterRequest {
        RegisterRequest {
            email: "test@example.com".to_string(),
            name: "Test User".to_string(),
            password: "password123".to_string(),
        }
    }

    #[tokio::test]
    async fn test_create_user_saves_to_db() {
        let pool = setup_test_db().await;
        let req = test_register_request();

        let user = create_user(&pool, &req)
            .await
            .expect("Failed to create user");

        assert_eq!(user.email, "test@example.com");
        assert_eq!(user.name, "Test User");
        assert!(!user.id.is_nil());
    }

    #[tokio::test]
    async fn test_create_user_hashes_password() {
        let pool = setup_test_db().await;
        let req = test_register_request();

        create_user(&pool, &req)
            .await
            .expect("Failed to create user");

        // Verify password hash is stored, not plain password
        let row: (String,) = sqlx::query_as("SELECT password_hash FROM users WHERE email = $1")
            .bind(&req.email)
            .fetch_one(&pool)
            .await
            .expect("Failed to fetch");

        assert!(row.0.starts_with("$argon2"));
        assert_ne!(row.0, req.password);
    }

    #[tokio::test]
    async fn test_create_user_duplicate_email_fails() {
        let pool = setup_test_db().await;
        let req = test_register_request();

        create_user(&pool, &req)
            .await
            .expect("First creation should succeed");
        let result = create_user(&pool, &req).await;

        // SQLite returns Database error with UNIQUE constraint violation
        // which gets converted to Conflict in IntoResponse
        assert!(
            matches!(result, Err(AppError::Database(_))),
            "Expected Database error for duplicate email, got: {:?}",
            result
        );
    }

    #[tokio::test]
    async fn test_get_user_returns_existing() {
        let pool = setup_test_db().await;
        let req = test_register_request();
        let created = create_user(&pool, &req).await.expect("Failed to create");

        let found = get_user(&pool, created.id)
            .await
            .expect("Failed to get user");

        assert_eq!(found.id, created.id);
        assert_eq!(found.email, "test@example.com");
    }

    #[tokio::test]
    async fn test_get_user_not_found() {
        let pool = setup_test_db().await;
        let random_id = Uuid::new_v4();

        let result = get_user(&pool, random_id).await;

        assert!(matches!(result, Err(AppError::NotFound(_))));
    }

    #[tokio::test]
    async fn test_get_user_by_email_returns_existing() {
        let pool = setup_test_db().await;
        let req = test_register_request();
        let created = create_user(&pool, &req).await.expect("Failed to create");

        let found = get_user_by_email(&pool, &req.email)
            .await
            .expect("Failed to get user");

        assert_eq!(found.id, created.id);
    }

    #[tokio::test]
    async fn test_get_user_by_email_not_found() {
        let pool = setup_test_db().await;

        let result = get_user_by_email(&pool, "nonexistent@example.com").await;

        assert!(matches!(result, Err(AppError::NotFound(_))));
    }

    #[tokio::test]
    async fn test_authenticate_user_with_correct_password() {
        let pool = setup_test_db().await;
        let req = test_register_request();
        let created = create_user(&pool, &req).await.expect("Failed to create");

        let user = authenticate_user(&pool, &req.email, &req.password)
            .await
            .expect("Auth should succeed");

        assert_eq!(user.id, created.id);
    }

    #[tokio::test]
    async fn test_authenticate_user_with_wrong_password() {
        let pool = setup_test_db().await;
        let req = test_register_request();
        create_user(&pool, &req).await.expect("Failed to create");

        let result = authenticate_user(&pool, &req.email, "wrong_password").await;

        assert!(matches!(result, Err(AppError::InvalidCredentials)));
    }

    #[tokio::test]
    async fn test_authenticate_user_nonexistent_email() {
        let pool = setup_test_db().await;

        let result = authenticate_user(&pool, "nonexistent@example.com", "password").await;

        assert!(matches!(result, Err(AppError::InvalidCredentials)));
    }
}
