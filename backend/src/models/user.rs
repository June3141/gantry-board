use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

/// User model for API responses (excludes password_hash)
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct User {
    pub id: Uuid,
    pub name: String,
    pub email: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Internal user model with password hash (for database operations)
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct UserWithPassword {
    pub id: String,
    pub name: String,
    pub email: String,
    pub password_hash: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl TryFrom<UserWithPassword> for User {
    type Error = uuid::Error;

    fn try_from(row: UserWithPassword) -> Result<Self, Self::Error> {
        Ok(User {
            id: row.id.parse()?,
            name: row.name,
            email: row.email,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
    }
}

/// Request to register a new user
#[derive(Debug, Deserialize, ToSchema, garde::Validate)]
pub struct RegisterRequest {
    #[garde(email)]
    pub email: String,
    #[garde(length(min = 1, max = 100))]
    pub name: String,
    #[garde(length(min = 8, max = 128))]
    pub password: String,
}

/// Request to login
#[derive(Debug, Deserialize, ToSchema, garde::Validate)]
pub struct LoginRequest {
    #[garde(email)]
    pub email: String,
    #[garde(length(min = 1))]
    pub password: String,
}

/// Response after successful authentication
#[derive(Debug, Serialize, ToSchema)]
pub struct AuthResponse {
    pub user: User,
}

/// Session model
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, sqlx::FromRow)]
pub struct Session {
    pub id: String,
    pub user_id: String,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub last_active_at: DateTime<Utc>,
}
