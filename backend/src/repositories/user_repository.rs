use sqlx::SqlitePool;
use uuid::Uuid;

use crate::error::{AppError, AppResult};
use crate::models::user::{User, UserWithPassword};

/// Row type for queries that don't include password_hash.
#[derive(sqlx::FromRow)]
struct UserRow {
    id: String,
    name: String,
    email: String,
    is_admin: bool,
    created_at: chrono::DateTime<chrono::Utc>,
    updated_at: chrono::DateTime<chrono::Utc>,
}

impl TryFrom<UserRow> for User {
    type Error = uuid::Error;

    fn try_from(row: UserRow) -> Result<Self, Self::Error> {
        Ok(User {
            id: row.id.parse()?,
            name: row.name,
            email: row.email,
            is_admin: row.is_admin,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
    }
}

/// Find a user by ID (includes password_hash for auth).
pub async fn find_by_id(pool: &SqlitePool, id: Uuid) -> AppResult<Option<UserWithPassword>> {
    let row = sqlx::query_as::<_, UserWithPassword>(
        r#"
        SELECT id, email, name, password_hash, is_admin, created_at, updated_at
        FROM users
        WHERE id = $1
        "#,
    )
    .bind(id.to_string())
    .fetch_optional(pool)
    .await?;

    Ok(row)
}

/// Find a user by email (includes password_hash for auth).
pub async fn find_by_email(pool: &SqlitePool, email: &str) -> AppResult<Option<UserWithPassword>> {
    let row = sqlx::query_as::<_, UserWithPassword>(
        r#"
        SELECT id, email, name, password_hash, is_admin, created_at, updated_at
        FROM users
        WHERE email = $1
        "#,
    )
    .bind(email)
    .fetch_optional(pool)
    .await?;

    Ok(row)
}

/// Escape LIKE meta-characters so that `%` and `_` in user input are treated
/// as literal characters, not wildcards.
fn escape_like(input: &str) -> String {
    input
        .replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_")
}

/// Search users by name or email (LIKE match).
pub async fn search(pool: &SqlitePool, query: &str, limit: i64) -> AppResult<Vec<User>> {
    let escaped = escape_like(query);
    let pattern = format!("%{escaped}%");
    let rows = sqlx::query_as::<_, UserRow>(
        r#"
        SELECT id, name, email, is_admin, created_at, updated_at
        FROM users
        WHERE name LIKE $1 ESCAPE '\' OR email LIKE $1 ESCAPE '\'
        ORDER BY name ASC
        LIMIT $2
        "#,
    )
    .bind(&pattern)
    .bind(limit)
    .fetch_all(pool)
    .await?;

    rows.into_iter()
        .map(|r| r.try_into())
        .collect::<Result<Vec<User>, _>>()
        .map_err(|e: uuid::Error| AppError::Internal(e.to_string()))
}
