use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use garde::Validate;
use uuid::Uuid;

use crate::auth::middleware::AuthUser;
use crate::error::{AppError, AppResult};
use crate::models::project::MemberRole;
use crate::models::project_invitation::{
    CreateInvitationRequest, CreateInvitationResponse, InvitationInfo, ProjectInvitation,
};
use crate::services::{authorization_service, invitation_service};
use crate::AppState;

#[utoipa::path(
    post,
    path = "/api/projects/{project_id}/invitations",
    params(("project_id" = Uuid, Path, description = "Project ID")),
    request_body = CreateInvitationRequest,
    responses(
        (status = 201, description = "Invitation created", body = CreateInvitationResponse),
        (status = 403, description = "Forbidden — admin or owner required"),
        (status = 404, description = "Project not found")
    ),
    tag = "invitations"
)]
pub async fn create_invitation(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(project_id): Path<Uuid>,
    Json(body): Json<CreateInvitationRequest>,
) -> AppResult<(StatusCode, Json<CreateInvitationResponse>)> {
    body.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;
    authorization_service::require_project_admin(&state.pool, auth.user_id, project_id).await?;

    let role = body.role.unwrap_or(MemberRole::Member);
    let (invitation, token) =
        invitation_service::create_invitation(&state.pool, project_id, auth.user_id, role).await?;

    let base_url = state
        .config
        .cors_origin
        .as_deref()
        .unwrap_or("http://localhost:5173");
    let invite_url = format!("{}/invite/{}", base_url, token);

    Ok((
        StatusCode::CREATED,
        Json(CreateInvitationResponse {
            invitation,
            token,
            invite_url,
        }),
    ))
}

#[utoipa::path(
    get,
    path = "/api/projects/{project_id}/invitations",
    params(("project_id" = Uuid, Path, description = "Project ID")),
    responses(
        (status = 200, description = "List invitations", body = Vec<ProjectInvitation>),
        (status = 403, description = "Forbidden — admin or owner required"),
        (status = 404, description = "Project not found")
    ),
    tag = "invitations"
)]
pub async fn list_invitations(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(project_id): Path<Uuid>,
) -> AppResult<Json<Vec<ProjectInvitation>>> {
    authorization_service::require_project_admin(&state.pool, auth.user_id, project_id).await?;
    let invitations = invitation_service::list_invitations(&state.pool, project_id).await?;
    Ok(Json(invitations))
}

#[utoipa::path(
    delete,
    path = "/api/projects/{project_id}/invitations/{invitation_id}",
    params(
        ("project_id" = Uuid, Path, description = "Project ID"),
        ("invitation_id" = Uuid, Path, description = "Invitation ID")
    ),
    responses(
        (status = 204, description = "Invitation deleted"),
        (status = 403, description = "Forbidden — admin or owner required"),
        (status = 404, description = "Invitation not found")
    ),
    tag = "invitations"
)]
pub async fn delete_invitation(
    State(state): State<AppState>,
    auth: AuthUser,
    Path((project_id, invitation_id)): Path<(Uuid, Uuid)>,
) -> AppResult<StatusCode> {
    authorization_service::require_project_admin(&state.pool, auth.user_id, project_id).await?;
    invitation_service::delete_invitation(&state.pool, project_id, invitation_id).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(
    get,
    path = "/api/invitations/{token}",
    params(("token" = String, Path, description = "Invitation token")),
    responses(
        (status = 200, description = "Invitation info", body = InvitationInfo),
        (status = 404, description = "Invalid token")
    ),
    tag = "invitations"
)]
pub async fn get_invitation_by_token(
    State(state): State<AppState>,
    Path(token): Path<String>,
) -> AppResult<Json<InvitationInfo>> {
    let info = invitation_service::get_invitation_by_token(&state.pool, &token).await?;
    Ok(Json(info))
}

#[utoipa::path(
    post,
    path = "/api/invitations/{token}/accept",
    params(("token" = String, Path, description = "Invitation token")),
    responses(
        (status = 200, description = "Invitation accepted", body = ProjectInvitation),
        (status = 400, description = "Invitation expired"),
        (status = 404, description = "Invalid token"),
        (status = 409, description = "Already accepted")
    ),
    tag = "invitations"
)]
pub async fn accept_invitation(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(token): Path<String>,
) -> AppResult<Json<ProjectInvitation>> {
    let invitation =
        invitation_service::accept_invitation(&state.pool, &token, auth.user_id).await?;
    Ok(Json(invitation))
}
