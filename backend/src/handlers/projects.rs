use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use garde::Validate;
use uuid::Uuid;

use crate::auth::middleware::AuthUser;
use crate::error::{AppError, AppResult};
use crate::models::project::{
    AddMemberRequest, CreateProjectRequest, MemberRole, Project, UpdateProjectRequest,
};
use crate::services::{authorization_service, member_service, project_service};
use crate::AppState;

#[utoipa::path(
    get,
    path = "/api/projects",
    responses(
        (status = 200, description = "List all projects", body = Vec<Project>)
    ),
    tag = "projects"
)]
pub async fn list_projects(
    State(state): State<AppState>,
    auth: AuthUser,
) -> AppResult<Json<Vec<Project>>> {
    let projects = {
        #[cfg(debug_assertions)]
        if auth.user_id.is_nil() {
            project_service::list_projects(&state.pool).await?
        } else {
            project_service::list_projects_for_user(&state.pool, auth.user_id).await?
        }
        #[cfg(not(debug_assertions))]
        project_service::list_projects_for_user(&state.pool, auth.user_id).await?
    };
    Ok(Json(projects))
}

#[utoipa::path(
    post,
    path = "/api/projects",
    request_body = CreateProjectRequest,
    responses(
        (status = 201, description = "Project created", body = Project)
    ),
    tag = "projects"
)]
pub async fn create_project(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(body): Json<CreateProjectRequest>,
) -> AppResult<(StatusCode, Json<Project>)> {
    body.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;
    let project = project_service::create_project(&state.pool, &body).await?;

    // Auto-add creator as owner (skip in auth_disabled mode for debug builds)
    let should_add_owner = {
        #[cfg(debug_assertions)]
        {
            !auth.user_id.is_nil()
        }
        #[cfg(not(debug_assertions))]
        {
            true
        }
    };
    if should_add_owner {
        member_service::add_member(
            &state.pool,
            project.id,
            &AddMemberRequest {
                user_id: auth.user_id,
                role: MemberRole::Owner,
            },
        )
        .await?;
    }

    Ok((StatusCode::CREATED, Json(project)))
}

#[utoipa::path(
    get,
    path = "/api/projects/{id}",
    params(("id" = Uuid, Path, description = "Project ID")),
    responses(
        (status = 200, description = "Project found", body = Project),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Project not found")
    ),
    tag = "projects"
)]
pub async fn get_project(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> AppResult<Json<Project>> {
    authorization_service::require_project_member(&state.pool, auth.user_id, id).await?;
    let project = project_service::get_project(&state.pool, id).await?;
    Ok(Json(project))
}

#[utoipa::path(
    patch,
    path = "/api/projects/{id}",
    params(("id" = Uuid, Path, description = "Project ID")),
    request_body = UpdateProjectRequest,
    responses(
        (status = 200, description = "Project updated", body = Project),
        (status = 403, description = "Forbidden - requires Admin or Owner"),
        (status = 404, description = "Project not found")
    ),
    tag = "projects"
)]
pub async fn update_project(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
    Json(body): Json<UpdateProjectRequest>,
) -> AppResult<Json<Project>> {
    authorization_service::require_project_admin(&state.pool, auth.user_id, id).await?;
    body.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;
    let project = project_service::update_project(&state.pool, id, &body).await?;
    Ok(Json(project))
}

#[utoipa::path(
    delete,
    path = "/api/projects/{id}",
    params(("id" = Uuid, Path, description = "Project ID")),
    responses(
        (status = 204, description = "Project deleted"),
        (status = 403, description = "Forbidden - requires Owner"),
        (status = 404, description = "Project not found")
    ),
    tag = "projects"
)]
pub async fn delete_project(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> AppResult<StatusCode> {
    authorization_service::require_project_owner(&state.pool, auth.user_id, id).await?;
    project_service::delete_project(&state.pool, id).await?;
    Ok(StatusCode::NO_CONTENT)
}
