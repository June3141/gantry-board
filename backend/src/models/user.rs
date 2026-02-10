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

/// Context for password validation — passes user-specific inputs to zxcvbn.
pub struct PasswordContext {
    pub email: String,
    pub name: String,
}

/// Request to register a new user
#[derive(Debug, Deserialize, ToSchema, garde::Validate)]
#[garde(context(PasswordContext))]
pub struct RegisterRequest {
    #[garde(email)]
    pub email: String,
    #[garde(length(min = 1, max = 100))]
    pub name: String,
    #[garde(length(min = 8, max = 128), custom(password_strength))]
    pub password: String,
}

impl RegisterRequest {
    pub fn password_context(&self) -> PasswordContext {
        PasswordContext {
            email: self.email.clone(),
            name: self.name.clone(),
        }
    }
}

fn password_strength(value: &str, context: &PasswordContext) -> garde::Result {
    let user_inputs: Vec<&str> = vec![&context.email, &context.name];
    let estimate = zxcvbn::zxcvbn(value, &user_inputs);
    if estimate.score() < zxcvbn::Score::Three {
        let warning = estimate
            .feedback()
            .as_ref()
            .and_then(|f| f.warning())
            .map(|w| format!("{w}"))
            .unwrap_or_default();
        let msg = if warning.is_empty() {
            "password is too weak".to_string()
        } else {
            format!("password is too weak: {warning}")
        };
        return Err(garde::Error::new(msg));
    }
    Ok(())
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

#[cfg(test)]
mod tests {
    use super::*;
    use garde::Validate;

    fn make_register(password: &str) -> RegisterRequest {
        RegisterRequest {
            email: "test@example.com".to_string(),
            name: "Test".to_string(),
            password: password.to_string(),
        }
    }

    #[test]
    fn test_weak_password_rejected() {
        let req = make_register("password123");
        let ctx = req.password_context();
        assert!(req.validate_with(&ctx).is_err());
    }

    #[test]
    fn test_common_password_rejected() {
        let req = make_register("qwerty1234");
        let ctx = req.password_context();
        assert!(req.validate_with(&ctx).is_err());
    }

    #[test]
    fn test_strong_password_accepted() {
        let req = make_register("c0rr3ct-h0rse-b@ttery-st@ple!");
        let ctx = req.password_context();
        assert!(req.validate_with(&ctx).is_ok());
    }

    #[test]
    fn test_passphrase_accepted() {
        let req = make_register("correct horse battery staple purple");
        let ctx = req.password_context();
        assert!(req.validate_with(&ctx).is_ok());
    }

    #[test]
    fn test_short_password_rejected() {
        let req = make_register("Ab1!");
        let ctx = req.password_context();
        assert!(req.validate_with(&ctx).is_err());
    }

    #[test]
    fn test_password_containing_email_rejected() {
        let req = RegisterRequest {
            email: "alice@example.com".to_string(),
            name: "Alice".to_string(),
            password: "alice@example.com!!".to_string(),
        };
        let ctx = req.password_context();
        assert!(req.validate_with(&ctx).is_err());
    }
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
