use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::Json;
use serde::{Deserialize, Serialize};

use crate::auth::middleware::AuthUser;
use crate::error::AppError;
use crate::services::audit_service;
use crate::AppState;

// ==========================================================================
// GET /api/admin/status (#287)
// ==========================================================================

#[derive(Debug, Serialize)]
pub struct DbStats {
    pub pool_size: u32,
    pub pool_idle: u32,
}

#[derive(Debug, Serialize)]
pub struct AdminStatusResponse {
    pub version: String,
    pub uptime_seconds: u64,
    pub db: DbStats,
    pub active_sessions: Vec<serde_json::Value>,
    pub realtime_connections: usize,
}

pub async fn admin_status(
    _user: AuthUser,
    State(state): State<AppState>,
) -> Result<Json<AdminStatusResponse>, AppError> {
    let pool_size = state.pool.size();
    let pool_idle = state.pool.num_idle() as u32;

    // Fetch active agent sessions
    let rows = sqlx::query_as::<_, (String, String, String, String, String)>(
        "SELECT id, task_id, agent_type, status, created_at FROM agent_sessions WHERE status IN ('running', 'paused', 'pending')",
    )
    .fetch_all(&state.pool)
    .await?;

    let active_sessions: Vec<serde_json::Value> = rows
        .into_iter()
        .map(|row| {
            serde_json::json!({
                "id": row.0,
                "task_id": row.1,
                "agent_type": row.2,
                "status": row.3,
                "created_at": row.4,
            })
        })
        .collect();

    let realtime_connections = state
        .connection_counter
        .load(std::sync::atomic::Ordering::Relaxed);

    Ok(Json(AdminStatusResponse {
        version: env!("CARGO_PKG_VERSION").to_string(),
        uptime_seconds: state.started_at.elapsed().as_secs(),
        db: DbStats {
            pool_size,
            pool_idle,
        },
        active_sessions,
        realtime_connections,
    }))
}

// ==========================================================================
// GET /api/admin/audit-log (#288)
// ==========================================================================

#[derive(Debug, Deserialize)]
pub struct AuditLogQuery {
    pub event_type: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

pub async fn audit_log(
    _user: AuthUser,
    State(state): State<AppState>,
    Query(params): Query<AuditLogQuery>,
) -> Result<(StatusCode, Json<audit_service::AuditLogResponse>), AppError> {
    let limit = params.limit.unwrap_or(50).min(200);
    let offset = params.offset.unwrap_or(0).max(0);

    let result =
        audit_service::list_events(&state.pool, params.event_type.as_deref(), limit, offset)
            .await?;

    Ok((StatusCode::OK, Json(result)))
}
