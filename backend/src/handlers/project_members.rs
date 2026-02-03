use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use garde::Validate;
use uuid::Uuid;

use crate::error::{AppError, AppResult};
use crate::models::project::{AddMemberRequest, ProjectMember, UpdateMemberRequest};
use crate::services::member_service;
use crate::AppState;

#[utoipa::path(
    get,
    path = "/api/projects/{project_id}/members",
    params(("project_id" = Uuid, Path, description = "Project ID")),
    responses(
        (status = 200, description = "List project members", body = Vec<ProjectMember>),
        (status = 404, description = "Project not found")
    ),
    tag = "project-members"
)]
pub async fn list_members(
    State(state): State<AppState>,
    Path(project_id): Path<Uuid>,
) -> AppResult<Json<Vec<ProjectMember>>> {
    let members = member_service::list_members(&state.pool, project_id).await?;
    Ok(Json(members))
}

#[utoipa::path(
    post,
    path = "/api/projects/{project_id}/members",
    params(("project_id" = Uuid, Path, description = "Project ID")),
    request_body = AddMemberRequest,
    responses(
        (status = 201, description = "Member added", body = ProjectMember),
        (status = 404, description = "Project not found")
    ),
    tag = "project-members"
)]
pub async fn add_member(
    State(state): State<AppState>,
    Path(project_id): Path<Uuid>,
    Json(body): Json<AddMemberRequest>,
) -> AppResult<(StatusCode, Json<ProjectMember>)> {
    body.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;
    let member = member_service::add_member(&state.pool, project_id, &body).await?;
    Ok((StatusCode::CREATED, Json(member)))
}

#[utoipa::path(
    get,
    path = "/api/projects/{project_id}/members/{user_id}",
    params(
        ("project_id" = Uuid, Path, description = "Project ID"),
        ("user_id" = Uuid, Path, description = "User ID")
    ),
    responses(
        (status = 200, description = "Member found", body = ProjectMember),
        (status = 404, description = "Member not found")
    ),
    tag = "project-members"
)]
pub async fn get_member(
    State(state): State<AppState>,
    Path((project_id, user_id)): Path<(Uuid, Uuid)>,
) -> AppResult<Json<ProjectMember>> {
    let member = member_service::get_member(&state.pool, project_id, user_id).await?;
    Ok(Json(member))
}

#[utoipa::path(
    patch,
    path = "/api/projects/{project_id}/members/{user_id}",
    params(
        ("project_id" = Uuid, Path, description = "Project ID"),
        ("user_id" = Uuid, Path, description = "User ID")
    ),
    request_body = UpdateMemberRequest,
    responses(
        (status = 200, description = "Member role updated", body = ProjectMember),
        (status = 404, description = "Member not found")
    ),
    tag = "project-members"
)]
pub async fn update_member(
    State(state): State<AppState>,
    Path((project_id, user_id)): Path<(Uuid, Uuid)>,
    Json(body): Json<UpdateMemberRequest>,
) -> AppResult<Json<ProjectMember>> {
    body.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;
    let member =
        member_service::update_member_role(&state.pool, project_id, user_id, &body).await?;
    Ok(Json(member))
}

#[utoipa::path(
    delete,
    path = "/api/projects/{project_id}/members/{user_id}",
    params(
        ("project_id" = Uuid, Path, description = "Project ID"),
        ("user_id" = Uuid, Path, description = "User ID")
    ),
    responses(
        (status = 204, description = "Member removed"),
        (status = 404, description = "Member not found")
    ),
    tag = "project-members"
)]
pub async fn remove_member(
    State(state): State<AppState>,
    Path((project_id, user_id)): Path<(Uuid, Uuid)>,
) -> AppResult<StatusCode> {
    member_service::remove_member(&state.pool, project_id, user_id).await?;
    Ok(StatusCode::NO_CONTENT)
}
