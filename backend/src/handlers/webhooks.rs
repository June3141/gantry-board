use std::sync::Arc;

use axum::body::Bytes;
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use hmac::{Hmac, Mac};
use sha2::Sha256;

use crate::error::{AppError, AppResult};
use crate::github::sync_engine::SyncEngine;
use crate::services::github_link_service;
use crate::AppState;

type HmacSha256 = Hmac<Sha256>;

/// Verify GitHub webhook signature (X-Hub-Signature-256).
fn verify_signature(secret: &str, payload: &[u8], signature_header: &str) -> bool {
    let Some(hex_sig) = signature_header.strip_prefix("sha256=") else {
        return false;
    };

    let Ok(expected) = hex::decode(hex_sig) else {
        return false;
    };

    let Ok(mut mac) = HmacSha256::new_from_slice(secret.as_bytes()) else {
        return false;
    };

    mac.update(payload);
    mac.verify_slice(&expected).is_ok()
}

/// `POST /api/webhooks/github` — receive GitHub webhook events.
///
/// This endpoint does NOT require authentication; instead it verifies
/// the `X-Hub-Signature-256` HMAC signature using the configured secret.
#[utoipa::path(
    post,
    path = "/api/webhooks/github",
    tag = "webhooks",
    request_body(content = String, content_type = "application/json"),
    responses(
        (status = 200, description = "Webhook processed"),
        (status = 401, description = "Invalid signature"),
        (status = 400, description = "Bad request")
    )
)]
pub async fn github_webhook(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> AppResult<StatusCode> {
    // 1. Verify signature
    let secret = state
        .config
        .github_webhook_secret
        .as_deref()
        .ok_or_else(|| AppError::Internal("webhook secret not configured".to_string()))?;

    let signature = headers
        .get("x-hub-signature-256")
        .and_then(|v| v.to_str().ok())
        .ok_or(AppError::Unauthorized)?;

    if !verify_signature(secret, &body, signature) {
        return Err(AppError::Unauthorized);
    }

    // 2. Route by event type
    let event = headers
        .get("x-github-event")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    match event {
        "ping" => {
            tracing::info!("received GitHub webhook ping");
            Ok(StatusCode::OK)
        }
        "issues" | "pull_request" => {
            handle_sync_event(&state, &body).await?;
            Ok(StatusCode::OK)
        }
        _ => {
            tracing::debug!(event, "ignoring unhandled GitHub webhook event");
            Ok(StatusCode::OK)
        }
    }
}

/// Parse the repo info from the webhook payload and trigger a sync.
async fn handle_sync_event(state: &AppState, body: &[u8]) -> AppResult<()> {
    let payload: serde_json::Value =
        serde_json::from_slice(body).map_err(|e| AppError::Validation(e.to_string()))?;

    let repo_owner = payload["repository"]["owner"]["login"]
        .as_str()
        .unwrap_or_default();
    let repo_name = payload["repository"]["name"].as_str().unwrap_or_default();

    if repo_owner.is_empty() || repo_name.is_empty() {
        tracing::warn!("webhook payload missing repository info");
        return Ok(());
    }

    // Find matching github_link
    let link = github_link_service::find_by_repo(&state.pool, repo_owner, repo_name).await?;

    let Some(link) = link else {
        tracing::debug!(repo_owner, repo_name, "no matching GitHub link for webhook");
        return Ok(());
    };

    let Some(ref github_client) = state.github_client else {
        tracing::warn!("GitHub client not available for webhook sync");
        return Ok(());
    };

    let engine = SyncEngine::new(Arc::clone(github_client), state.pool.clone());

    match engine.sync_project(&link).await {
        Ok(result) => {
            if result.pushed > 0 || result.pulled > 0 {
                tracing::info!(
                    project_id = %result.project_id,
                    pushed = result.pushed,
                    pulled = result.pulled,
                    "webhook-triggered sync completed"
                );
            }
            state
                .sse_hub
                .broadcast(crate::sse::event::SseEvent::github_sync_completed(result));
        }
        Err(e) => {
            tracing::warn!(error = %e, "webhook-triggered sync failed");
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_verify_valid_signature() {
        let secret = "test-secret";
        let payload = b"hello world";
        // Pre-computed HMAC-SHA256 of "hello world" with key "test-secret"
        let mut mac = HmacSha256::new_from_slice(secret.as_bytes()).unwrap();
        mac.update(payload);
        let sig = hex::encode(mac.finalize().into_bytes());
        let header = format!("sha256={sig}");

        assert!(verify_signature(secret, payload, &header));
    }

    #[test]
    fn test_reject_invalid_signature() {
        assert!(!verify_signature("secret", b"payload", "sha256=deadbeef"));
    }

    #[test]
    fn test_reject_missing_prefix() {
        assert!(!verify_signature("secret", b"payload", "invalid"));
    }
}
