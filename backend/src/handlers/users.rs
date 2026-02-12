use axum::extract::{Query, State};
use axum::Json;
use serde::Deserialize;

use crate::auth::middleware::AuthUser;
use crate::error::AppResult;
use crate::models::pagination;
use crate::models::user::User;
use crate::services::user_service;
use crate::AppState;

#[derive(Debug, Deserialize)]
pub struct SearchUsersQuery {
    pub q: Option<String>,
    #[serde(default = "pagination::default_limit")]
    pub limit: i64,
}

#[utoipa::path(
    get,
    path = "/api/users",
    params(
        ("q" = Option<String>, Query, description = "Search term for name or email"),
        ("limit" = Option<i64>, Query, description = "Maximum number of results (default 50)"),
    ),
    responses(
        (status = 200, description = "List of matching users", body = Vec<User>),
        (status = 401, description = "Unauthorized")
    ),
    tag = "users"
)]
pub async fn search_users(
    State(state): State<AppState>,
    _auth: AuthUser,
    Query(params): Query<SearchUsersQuery>,
) -> AppResult<Json<Vec<User>>> {
    let query = params.q.as_deref().unwrap_or("");
    let limit = params.limit.clamp(1, 100);
    let users = user_service::search_users(&state.pool, query, limit).await?;
    Ok(Json(users))
}
