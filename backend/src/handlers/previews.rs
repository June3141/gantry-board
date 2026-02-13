use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use uuid::Uuid;

use crate::auth::middleware::AuthUser;
use crate::error::{AppError, AppResult};
use crate::models::docker_preview::{CreatePreviewRequest, DockerPreview};
use crate::services::{preview_service, worktree_service};
use crate::sse::event::SseEvent;
use crate::AppState;

#[utoipa::path(
    post,
    path = "/api/previews",
    request_body = CreatePreviewRequest,
    responses(
        (status = 201, description = "Preview created", body = DockerPreview),
        (status = 404, description = "Worktree not found"),
        (status = 409, description = "Preview already exists for this worktree")
    ),
    tag = "previews"
)]
pub async fn create_preview(
    State(state): State<AppState>,
    _auth: AuthUser,
    Json(body): Json<CreatePreviewRequest>,
) -> AppResult<(StatusCode, Json<DockerPreview>)> {
    let worktree_name = body.worktree_name.trim().to_string();

    // Validate worktree exists
    let repo_path = state.config.repo_path();
    let name = worktree_name.clone();
    tokio::task::spawn_blocking(move || worktree_service::get_worktree(&repo_path, &name))
        .await
        .map_err(|e| AppError::Internal(e.to_string()))??;

    let preview = preview_service::create_preview(&state.pool, &worktree_name).await?;

    state
        .sse_hub
        .broadcast(SseEvent::preview_status_changed(preview.clone()));

    Ok((StatusCode::CREATED, Json(preview)))
}

#[utoipa::path(
    get,
    path = "/api/previews",
    responses(
        (status = 200, description = "List all previews", body = Vec<DockerPreview>),
    ),
    tag = "previews"
)]
pub async fn list_previews(
    State(state): State<AppState>,
    _auth: AuthUser,
) -> AppResult<Json<Vec<DockerPreview>>> {
    let previews = preview_service::list_previews(&state.pool).await?;
    Ok(Json(previews))
}

#[utoipa::path(
    get,
    path = "/api/previews/{id}",
    params(("id" = Uuid, Path, description = "Preview ID")),
    responses(
        (status = 200, description = "Preview found", body = DockerPreview),
        (status = 404, description = "Preview not found")
    ),
    tag = "previews"
)]
pub async fn get_preview(
    State(state): State<AppState>,
    _auth: AuthUser,
    Path(id): Path<Uuid>,
) -> AppResult<Json<DockerPreview>> {
    let preview = preview_service::get_preview(&state.pool, id).await?;
    Ok(Json(preview))
}

#[utoipa::path(
    delete,
    path = "/api/previews/{id}",
    params(("id" = Uuid, Path, description = "Preview ID")),
    responses(
        (status = 204, description = "Preview deleted"),
        (status = 404, description = "Preview not found")
    ),
    tag = "previews"
)]
pub async fn delete_preview(
    State(state): State<AppState>,
    _auth: AuthUser,
    Path(id): Path<Uuid>,
) -> AppResult<StatusCode> {
    // Cleanup container before deleting DB record (cleanup reads the record)
    if let Some(ref pm) = state.preview_manager {
        let _ = pm.cleanup(id).await;
    }

    preview_service::delete_preview(&state.pool, id).await?;

    state.sse_hub.broadcast(SseEvent::preview_deleted(id));

    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(
    post,
    path = "/api/previews/{id}/start",
    params(("id" = Uuid, Path, description = "Preview ID")),
    responses(
        (status = 202, description = "Build and start initiated"),
        (status = 404, description = "Preview not found")
    ),
    tag = "previews"
)]
pub async fn start_preview(
    State(state): State<AppState>,
    _auth: AuthUser,
    Path(id): Path<Uuid>,
) -> AppResult<StatusCode> {
    // Verify preview exists
    preview_service::get_preview(&state.pool, id).await?;

    let pm = state
        .preview_manager
        .clone()
        .ok_or_else(|| AppError::Internal("Docker is not available".to_string()))?;

    tokio::spawn(async move {
        if let Err(e) = pm.build_and_start(id).await {
            tracing::error!(%id, %e, "preview build_and_start failed");
        }
    });

    Ok(StatusCode::ACCEPTED)
}

#[utoipa::path(
    post,
    path = "/api/previews/{id}/stop",
    params(("id" = Uuid, Path, description = "Preview ID")),
    responses(
        (status = 200, description = "Preview stopped", body = DockerPreview),
        (status = 404, description = "Preview not found")
    ),
    tag = "previews"
)]
pub async fn stop_preview(
    State(state): State<AppState>,
    _auth: AuthUser,
    Path(id): Path<Uuid>,
) -> AppResult<Json<DockerPreview>> {
    if let Some(ref pm) = state.preview_manager {
        let preview = pm.stop(id).await?;
        Ok(Json(preview))
    } else {
        // No Docker available: just update status in DB
        let mut conn = state.pool.acquire().await?;
        let preview = preview_service::update_status_tx(
            &mut conn,
            id,
            crate::models::docker_preview::PreviewStatus::Stopped,
            None,
        )
        .await?;
        state
            .sse_hub
            .broadcast(SseEvent::preview_status_changed(preview.clone()));
        Ok(Json(preview))
    }
}

#[utoipa::path(
    post,
    path = "/api/previews/{id}/restart",
    params(("id" = Uuid, Path, description = "Preview ID")),
    responses(
        (status = 202, description = "Restart initiated"),
        (status = 404, description = "Preview not found")
    ),
    tag = "previews"
)]
pub async fn restart_preview(
    State(state): State<AppState>,
    _auth: AuthUser,
    Path(id): Path<Uuid>,
) -> AppResult<StatusCode> {
    // Verify preview exists
    preview_service::get_preview(&state.pool, id).await?;

    let pm = state
        .preview_manager
        .clone()
        .ok_or_else(|| AppError::Internal("Docker is not available".to_string()))?;

    tokio::spawn(async move {
        // Stop first, then rebuild
        let _ = pm.stop(id).await;
        if let Err(e) = pm.build_and_start(id).await {
            tracing::error!(%id, %e, "preview restart failed");
        }
    });

    Ok(StatusCode::ACCEPTED)
}
