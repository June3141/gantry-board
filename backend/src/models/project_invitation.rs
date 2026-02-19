use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use super::project::MemberRole;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ProjectInvitation {
    pub id: Uuid,
    pub project_id: Uuid,
    pub invited_by: Uuid,
    pub invited_by_name: String,
    pub project_name: String,
    pub role: MemberRole,
    pub expires_at: DateTime<Utc>,
    pub accepted_at: Option<DateTime<Utc>>,
    pub accepted_by: Option<Uuid>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, ToSchema, garde::Validate)]
pub struct CreateInvitationRequest {
    #[garde(skip)]
    pub role: Option<MemberRole>,
}

/// Response when creating an invitation — includes the raw token (shown once).
#[derive(Debug, Serialize, ToSchema)]
pub struct CreateInvitationResponse {
    pub invitation: ProjectInvitation,
    pub token: String,
    pub invite_url: String,
}

/// Public invitation info (no sensitive data).
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct InvitationInfo {
    pub id: Uuid,
    pub project_name: String,
    pub invited_by_name: String,
    pub role: MemberRole,
    pub expires_at: DateTime<Utc>,
    pub expired: bool,
    pub accepted: bool,
}
