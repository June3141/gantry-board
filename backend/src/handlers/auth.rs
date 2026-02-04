use axum::extract::State;
use axum::http::{header, StatusCode};
use axum::response::IntoResponse;
use axum::Json;
use garde::Validate;

use crate::auth::middleware::{create_session_cookie, delete_session_cookie, AuthUser};
use crate::error::{AppError, AppResult};
use crate::models::user::{AuthResponse, LoginRequest, RegisterRequest, User};
use crate::services::{session_service, user_service};
use crate::AppState;

#[utoipa::path(
    post,
    path = "/api/auth/register",
    request_body = RegisterRequest,
    responses(
        (status = 201, description = "User registered", body = AuthResponse),
        (status = 400, description = "Validation error"),
        (status = 409, description = "Email already exists")
    ),
    tag = "auth"
)]
pub async fn register(
    State(state): State<AppState>,
    Json(body): Json<RegisterRequest>,
) -> AppResult<impl IntoResponse> {
    body.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;

    let user = user_service::create_user(&state.pool, &body).await?;

    // Create session for the new user
    let session =
        session_service::create_session(&state.pool, user.id, state.config.session_duration_hours)
            .await?;

    let session_id = session
        .id
        .parse()
        .map_err(|_| AppError::Internal("failed to parse session id".to_string()))?;

    let cookie = create_session_cookie(session_id, state.config.cookie_secure);

    let response = AuthResponse { user };

    Ok((
        StatusCode::CREATED,
        [(header::SET_COOKIE, cookie)],
        Json(response),
    ))
}

#[utoipa::path(
    post,
    path = "/api/auth/login",
    request_body = LoginRequest,
    responses(
        (status = 200, description = "Login successful", body = AuthResponse),
        (status = 401, description = "Invalid credentials")
    ),
    tag = "auth"
)]
pub async fn login(
    State(state): State<AppState>,
    Json(body): Json<LoginRequest>,
) -> AppResult<impl IntoResponse> {
    body.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;

    let user = user_service::authenticate_user(&state.pool, &body.email, &body.password).await?;

    // Create session
    let session =
        session_service::create_session(&state.pool, user.id, state.config.session_duration_hours)
            .await?;

    let session_id = session
        .id
        .parse()
        .map_err(|_| AppError::Internal("failed to parse session id".to_string()))?;

    let cookie = create_session_cookie(session_id, state.config.cookie_secure);

    let response = AuthResponse { user };

    Ok((
        StatusCode::OK,
        [(header::SET_COOKIE, cookie)],
        Json(response),
    ))
}

#[utoipa::path(
    post,
    path = "/api/auth/logout",
    responses(
        (status = 204, description = "Logged out"),
        (status = 401, description = "Not authenticated")
    ),
    tag = "auth"
)]
pub async fn logout(State(state): State<AppState>, auth: AuthUser) -> AppResult<impl IntoResponse> {
    // Delete session from database
    session_service::delete_session(&state.pool, auth.session_id).await?;

    // Clear cookie
    let cookie = delete_session_cookie();

    Ok((StatusCode::NO_CONTENT, [(header::SET_COOKIE, cookie)]))
}

#[utoipa::path(
    get,
    path = "/api/auth/me",
    responses(
        (status = 200, description = "Current user", body = User),
        (status = 401, description = "Not authenticated")
    ),
    tag = "auth"
)]
pub async fn me(State(state): State<AppState>, auth: AuthUser) -> AppResult<Json<User>> {
    let user = user_service::get_user(&state.pool, auth.user_id).await?;
    Ok(Json(user))
}
