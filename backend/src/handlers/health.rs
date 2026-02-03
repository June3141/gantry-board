use axum::http::StatusCode;

#[utoipa::path(
    get,
    path = "/health",
    tag = "health",
    responses(
        (status = 200, description = "Service is healthy", body = String)
    )
)]
pub async fn health_check() -> (StatusCode, &'static str) {
    (StatusCode::OK, "ok")
}
