use axum::extract::State;
use axum::http::StatusCode;
use axum::middleware::Next;
use axum::response::Response;

use crate::AppState;

/// Middleware that validates the Host header against a configured allow-list.
/// When `allowed_hosts` is empty, validation is skipped.
pub async fn validate_host_header(
    State(state): State<AppState>,
    request: axum::extract::Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let allowed = &state.config.allowed_hosts;

    // Skip validation when no hosts are configured
    if allowed.is_empty() {
        return Ok(next.run(request).await);
    }

    let host = request
        .headers()
        .get("host")
        .and_then(|h| h.to_str().ok())
        .ok_or_else(|| {
            tracing::warn!("request missing Host header");
            StatusCode::BAD_REQUEST
        })?;

    if !allowed.iter().any(|h| h == host) {
        tracing::warn!(host, "rejected request with invalid Host header");
        return Err(StatusCode::BAD_REQUEST);
    }

    Ok(next.run(request).await)
}
