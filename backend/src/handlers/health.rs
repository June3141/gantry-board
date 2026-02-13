use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use serde::Serialize;
use utoipa::ToSchema;

use crate::AppState;

#[derive(Debug, Serialize, ToSchema)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
    pub db: String,
    pub uptime_seconds: u64,
}

#[utoipa::path(
    get,
    path = "/health",
    tag = "health",
    responses(
        (status = 200, description = "Service health status", body = HealthResponse)
    )
)]
pub async fn health_check(State(state): State<AppState>) -> Json<HealthResponse> {
    let db_status = match sqlx::query("SELECT 1").execute(&state.pool).await {
        Ok(_) => "connected",
        Err(_) => "error",
    };

    let status = if db_status == "connected" {
        "ok"
    } else {
        "degraded"
    };

    Json(HealthResponse {
        status: status.to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        db: db_status.to_string(),
        uptime_seconds: state.started_at.elapsed().as_secs(),
    })
}

#[derive(Debug, Serialize, ToSchema)]
pub struct LivenessResponse {
    pub status: String,
}

/// Liveness probe — always returns 200 if the process is alive.
#[utoipa::path(
    get,
    path = "/health/live",
    tag = "health",
    responses(
        (status = 200, description = "Process is alive", body = LivenessResponse)
    )
)]
pub async fn liveness() -> Json<LivenessResponse> {
    Json(LivenessResponse {
        status: "alive".to_string(),
    })
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ReadinessResponse {
    pub status: String,
    pub db: String,
}

/// Readiness probe — checks database connectivity.
#[utoipa::path(
    get,
    path = "/health/ready",
    tag = "health",
    responses(
        (status = 200, description = "Service is ready", body = ReadinessResponse),
        (status = 503, description = "Service is not ready", body = ReadinessResponse)
    )
)]
pub async fn readiness(State(state): State<AppState>) -> (StatusCode, Json<ReadinessResponse>) {
    let db_ok = sqlx::query("SELECT 1").execute(&state.pool).await.is_ok();

    let status_code = if db_ok {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };

    (
        status_code,
        Json(ReadinessResponse {
            status: if db_ok { "ready" } else { "not_ready" }.to_string(),
            db: if db_ok { "connected" } else { "error" }.to_string(),
        }),
    )
}
