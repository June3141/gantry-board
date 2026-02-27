use std::sync::Arc;

use axum::body::Bytes;
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use hmac::{Hmac, Mac};
use sha2::Sha256;

use crate::error::{AppError, AppResult};
use crate::github::sync_engine::SyncEngine;
use crate::services::{agent_session_service, github_link_service, worktree_service};
use crate::AppState;

type HmacSha256 = Hmac<Sha256>;

/// Verify GitHub webhook signature (X-Hub-Signature-256).
///
/// HMAC computation is performed before parsing the signature header to avoid
/// timing side-channels: an early return on format validation (before touching
/// the secret) would leak whether the header *format* was valid, potentially
/// aiding targeted attacks.
fn verify_signature(secret: &str, payload: &[u8], signature_header: &str) -> bool {
    // Compute HMAC first to ensure constant-time behaviour regardless of
    // format-validation outcome.
    let Ok(mut mac) = HmacSha256::new_from_slice(secret.as_bytes()) else {
        return false;
    };
    mac.update(payload);

    let Some(hex_sig) = signature_header.strip_prefix("sha256=") else {
        return false;
    };

    let Ok(expected) = hex::decode(hex_sig) else {
        return false;
    };

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
        .ok_or(AppError::Unauthorized)?;

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
        "issues" => {
            handle_sync_event(&state, &body).await?;
            Ok(StatusCode::OK)
        }
        "pull_request" => {
            handle_sync_event(&state, &body).await?;
            handle_pr_merge_cleanup(&state, &body).await;
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
                .broadcast(crate::realtime::event::SseEvent::github_sync_completed(
                    result,
                ));
        }
        Err(e) => {
            tracing::warn!(error = %e, "webhook-triggered sync failed");
        }
    }

    Ok(())
}

/// Clean up worktrees when a PR is merged.
///
/// On `action=closed` + `merged=true`, finds the task linked to this PR
/// and deletes any remaining worktrees created by agent sessions.
async fn handle_pr_merge_cleanup(state: &AppState, body: &[u8]) {
    let payload: serde_json::Value = match serde_json::from_slice(body) {
        Ok(v) => v,
        Err(_) => return,
    };

    let action = payload["action"].as_str().unwrap_or_default();
    let merged = payload["pull_request"]["merged"].as_bool().unwrap_or(false);

    if action != "closed" || !merged {
        return;
    }

    let pr_number = match payload["pull_request"]["number"].as_i64() {
        Some(n) => n,
        None => return,
    };

    let repo_owner = payload["repository"]["owner"]["login"]
        .as_str()
        .unwrap_or_default();
    let repo_name = payload["repository"]["name"].as_str().unwrap_or_default();

    if repo_owner.is_empty() || repo_name.is_empty() {
        return;
    }

    // Find the github_link for this repo
    let link = match github_link_service::find_by_repo(&state.pool, repo_owner, repo_name).await {
        Ok(Some(link)) => link,
        _ => return,
    };

    // Find tasks linked to this PR
    let task_ids = match crate::services::github_pr_service::find_task_ids_by_pr(
        &state.pool,
        link.id,
        pr_number,
    )
    .await
    {
        Ok(ids) => ids,
        Err(e) => {
            tracing::warn!(error = %e, "failed to find tasks for merged PR");
            return;
        }
    };

    let repo_path = state.config.repo_path();
    for task_id in task_ids {
        let sessions =
            match agent_session_service::list_sessions_with_worktrees(&state.pool, task_id).await {
                Ok(s) => s,
                Err(e) => {
                    tracing::warn!(error = %e, %task_id, "failed to list sessions with worktrees");
                    continue;
                }
            };

        for session in sessions {
            let Some(ref wt_name) = session.worktree_name else {
                continue;
            };
            let name = wt_name.clone();
            let path = repo_path.clone();
            match tokio::task::spawn_blocking(move || {
                worktree_service::delete_worktree(&path, &name)
            })
            .await
            {
                Ok(Ok(())) => {
                    tracing::info!(
                        worktree = %wt_name,
                        %task_id,
                        pr_number,
                        "cleaned up worktree after PR merge"
                    );
                    // Clear worktree_name so future events don't re-attempt deletion
                    if let Err(e) =
                        agent_session_service::clear_worktree_name(&state.pool, session.id).await
                    {
                        tracing::warn!(error = %e, session_id = %session.id, "failed to clear worktree_name");
                    }
                }
                Ok(Err(e)) => {
                    let msg = e.to_string();
                    if msg.contains("not found") || msg.contains("does not exist") {
                        tracing::debug!(
                            error = %e,
                            worktree = %wt_name,
                            "worktree already removed or not found"
                        );
                    } else {
                        tracing::warn!(
                            error = %e,
                            worktree = %wt_name,
                            "worktree cleanup failed"
                        );
                    }
                }
                Err(e) => {
                    tracing::warn!(error = %e, "worktree cleanup task panicked");
                }
            }
        }
    }
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

    #[test]
    fn test_reject_empty_signature_header() {
        assert!(!verify_signature("secret", b"payload", ""));
    }

    #[test]
    fn test_reject_invalid_hex_after_prefix() {
        assert!(!verify_signature("secret", b"payload", "sha256=not-hex!!"));
    }

    #[test]
    fn test_reject_truncated_signature() {
        // Valid hex but wrong length — HMAC comparison should fail
        assert!(!verify_signature("secret", b"payload", "sha256=abcd"));
    }

    #[test]
    fn test_empty_payload_with_valid_signature() {
        let secret = "test-secret";
        let payload = b"";
        let mut mac = HmacSha256::new_from_slice(secret.as_bytes()).unwrap();
        mac.update(payload);
        let sig = hex::encode(mac.finalize().into_bytes());
        let header = format!("sha256={sig}");

        assert!(verify_signature(secret, payload, &header));
    }
}
