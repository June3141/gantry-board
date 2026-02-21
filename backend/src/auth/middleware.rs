use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use axum_extra::extract::CookieJar;
use uuid::Uuid;

use crate::error::AppError;
use crate::services::session_service;
use crate::AppState;

pub const SESSION_COOKIE_NAME: &str = "gantry_session";

/// Stable user/session ID used when authentication is bypassed in debug builds
/// (`GANTRY_AUTH_DISABLED=true`). Integration tests seed a user row with this ID
/// so that foreign-key constraints are satisfied even without a real login flow.
#[cfg(debug_assertions)]
pub const DEBUG_USER_ID: Uuid = Uuid::nil();

/// Authenticated user extracted from session cookie
#[derive(Debug, Clone)]
pub struct AuthUser {
    pub user_id: Uuid,
    pub session_id: Uuid,
}

impl FromRequestParts<AppState> for AuthUser {
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        // Auth bypass for development — compiled out of release builds.
        // Uses DEBUG_USER_ID (nil UUID) so that integration tests can seed a
        // matching user row and satisfy foreign-key constraints.
        #[cfg(debug_assertions)]
        if state.config.auth_disabled {
            return Ok(AuthUser {
                user_id: DEBUG_USER_ID,
                session_id: DEBUG_USER_ID,
            });
        }

        // Extract cookies
        let cookies = CookieJar::from_request_parts(parts, state)
            .await
            .map_err(|_| AppError::Unauthorized)?;

        // Get session cookie
        let session_cookie = cookies
            .get(SESSION_COOKIE_NAME)
            .ok_or(AppError::Unauthorized)?;

        // Parse session ID
        let session_id: Uuid = session_cookie
            .value()
            .parse()
            .map_err(|_| AppError::Unauthorized)?;

        // Validate session
        let session = session_service::validate_session(&state.pool, session_id).await?;

        // Parse user_id from session
        let user_id: Uuid = session
            .user_id
            .parse()
            .map_err(|_| AppError::Internal("invalid user_id in session".to_string()))?;

        Ok(AuthUser {
            user_id,
            session_id,
        })
    }
}

/// Optional authentication - returns None if not authenticated
#[derive(Debug, Clone)]
pub struct MaybeAuthUser(pub Option<AuthUser>);

impl FromRequestParts<AppState> for MaybeAuthUser {
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        match AuthUser::from_request_parts(parts, state).await {
            Ok(user) => Ok(MaybeAuthUser(Some(user))),
            Err(_) => Ok(MaybeAuthUser(None)),
        }
    }
}

/// Create a session cookie value with Max-Age derived from session duration
pub fn create_session_cookie(
    session_id: Uuid,
    secure: bool,
    session_duration_hours: u64,
) -> String {
    let max_age_seconds = session_duration_hours * 3600;
    let mut cookie = format!("{}={}", SESSION_COOKIE_NAME, session_id);

    cookie.push_str("; Path=/");
    cookie.push_str("; HttpOnly");
    cookie.push_str("; SameSite=Strict");

    if secure {
        cookie.push_str("; Secure");
    }

    cookie.push_str(&format!("; Max-Age={}", max_age_seconds));

    cookie
}

/// Create an expired session cookie (for logout)
pub fn delete_session_cookie(secure: bool) -> String {
    let mut cookie = format!(
        "{}=; Path=/; HttpOnly; SameSite=Strict; Max-Age=0",
        SESSION_COOKIE_NAME
    );
    if secure {
        cookie.push_str("; Secure");
    }
    cookie
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_session_cookie_without_secure() {
        let session_id = Uuid::new_v4();
        let cookie = create_session_cookie(session_id, false, 168);

        assert!(cookie.contains(&session_id.to_string()));
        assert!(cookie.contains("HttpOnly"));
        assert!(cookie.contains("SameSite=Strict"));
        assert!(!cookie.contains("Secure"));
        assert!(cookie.contains("Max-Age=604800")); // 168h * 3600
    }

    #[test]
    fn test_create_session_cookie_with_secure() {
        let session_id = Uuid::new_v4();
        let cookie = create_session_cookie(session_id, true, 168);

        assert!(cookie.contains("Secure"));
    }

    #[test]
    fn test_create_session_cookie_uses_samesite_strict() {
        let session_id = Uuid::new_v4();
        let cookie = create_session_cookie(session_id, false, 1);

        assert!(cookie.contains("SameSite=Strict"));
        assert!(!cookie.contains("SameSite=Lax"));
    }

    #[test]
    fn test_create_session_cookie_max_age_matches_duration() {
        let session_id = Uuid::new_v4();
        // 24 hours = 86400 seconds
        let cookie = create_session_cookie(session_id, false, 24);
        assert!(cookie.contains("Max-Age=86400"));

        // 1 hour = 3600 seconds
        let cookie = create_session_cookie(session_id, false, 1);
        assert!(cookie.contains("Max-Age=3600"));
    }

    #[test]
    fn test_delete_session_cookie_includes_samesite_strict() {
        let cookie = delete_session_cookie(false);

        assert!(cookie.contains("SameSite=Strict"));
        assert!(!cookie.contains("SameSite=Lax"));
        assert!(cookie.contains("Max-Age=0"));
        assert!(cookie.contains(SESSION_COOKIE_NAME));
    }

    #[test]
    fn test_delete_session_cookie_includes_secure_flag_when_enabled() {
        let cookie = delete_session_cookie(true);

        assert!(cookie.contains("Secure"));
        assert!(cookie.contains("SameSite=Strict"));
        assert!(cookie.contains("Max-Age=0"));
    }

    #[test]
    fn test_delete_session_cookie_excludes_secure_flag_when_disabled() {
        let cookie = delete_session_cookie(false);

        assert!(!cookie.contains("Secure"));
    }

    /// Issue #278: DEBUG_USER_ID must be the nil UUID and be available only in debug builds
    #[cfg(debug_assertions)]
    #[test]
    fn test_debug_user_id_is_nil_uuid() {
        assert_eq!(DEBUG_USER_ID, Uuid::nil());
        assert!(DEBUG_USER_ID.is_nil());
    }
}
