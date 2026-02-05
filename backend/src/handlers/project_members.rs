use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use uuid::Uuid;

use crate::auth::middleware::AuthUser;
use crate::error::{AppError, AppResult};
use crate::models::project::{AddMemberRequest, MemberRole, ProjectMember, UpdateMemberRequest};
use crate::services::{authorization_service, member_service, project_service};
use crate::AppState;

#[utoipa::path(
    get,
    path = "/api/projects/{project_id}/members",
    params(("project_id" = Uuid, Path, description = "Project ID")),
    responses(
        (status = 200, description = "List project members", body = Vec<ProjectMember>),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Project not found")
    ),
    tag = "project-members"
)]
pub async fn list_members(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(project_id): Path<Uuid>,
) -> AppResult<Json<Vec<ProjectMember>>> {
    project_service::get_project(&state.pool, project_id).await?;
    authorization_service::require_project_member(&state.pool, auth.user_id, project_id).await?;
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
        (status = 403, description = "Forbidden - requires Admin or Owner"),
        (status = 404, description = "Project not found")
    ),
    tag = "project-members"
)]
pub async fn add_member(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(project_id): Path<Uuid>,
    Json(body): Json<AddMemberRequest>,
) -> AppResult<(StatusCode, Json<ProjectMember>)> {
    project_service::get_project(&state.pool, project_id).await?;
    authorization_service::require_project_admin(&state.pool, auth.user_id, project_id).await?;
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
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Member not found")
    ),
    tag = "project-members"
)]
pub async fn get_member(
    State(state): State<AppState>,
    auth: AuthUser,
    Path((project_id, user_id)): Path<(Uuid, Uuid)>,
) -> AppResult<Json<ProjectMember>> {
    authorization_service::require_project_member(&state.pool, auth.user_id, project_id).await?;
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
        (status = 403, description = "Forbidden - requires Admin or Owner"),
        (status = 404, description = "Member not found")
    ),
    tag = "project-members"
)]
pub async fn update_member(
    State(state): State<AppState>,
    auth: AuthUser,
    Path((project_id, user_id)): Path<(Uuid, Uuid)>,
    Json(body): Json<UpdateMemberRequest>,
) -> AppResult<Json<ProjectMember>> {
    authorization_service::require_project_admin(&state.pool, auth.user_id, project_id).await?;

    // Prevent downgrading the last owner
    let target = member_service::get_member(&state.pool, project_id, user_id).await?;
    if matches!(target.role, MemberRole::Owner) && !matches!(body.role, MemberRole::Owner) {
        let owner_count = authorization_service::count_owners(&state.pool, project_id).await?;
        if owner_count <= 1 {
            return Err(AppError::Validation(
                "cannot downgrade the last owner".to_string(),
            ));
        }
    }

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
        (status = 403, description = "Forbidden - requires Admin or Owner"),
        (status = 404, description = "Member not found")
    ),
    tag = "project-members"
)]
pub async fn remove_member(
    State(state): State<AppState>,
    auth: AuthUser,
    Path((project_id, user_id)): Path<(Uuid, Uuid)>,
) -> AppResult<StatusCode> {
    authorization_service::require_project_admin(&state.pool, auth.user_id, project_id).await?;

    // Prevent removing the last owner
    let target = member_service::get_member(&state.pool, project_id, user_id).await?;
    if matches!(target.role, MemberRole::Owner) {
        let owner_count = authorization_service::count_owners(&state.pool, project_id).await?;
        if owner_count <= 1 {
            return Err(AppError::Validation(
                "cannot remove the last owner".to_string(),
            ));
        }
    }

    member_service::remove_member(&state.pool, project_id, user_id).await?;
    Ok(StatusCode::NO_CONTENT)
}
