use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TaskComment {
    pub id: Uuid,
    pub task_id: Uuid,
    pub user_id: Uuid,
    pub user_name: String,
    pub content: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, ToSchema, garde::Validate)]
pub struct CreateCommentRequest {
    #[garde(length(min = 1, max = 10000))]
    pub content: String,
}

#[derive(Debug, Deserialize, ToSchema, garde::Validate)]
pub struct UpdateCommentRequest {
    #[garde(length(min = 1, max = 10000))]
    pub content: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use garde::Validate;

    #[test]
    fn test_create_comment_request_validates_content() {
        let req = CreateCommentRequest {
            content: "This is a valid comment".to_string(),
        };
        assert!(req.validate().is_ok());
    }

    #[test]
    fn test_create_comment_request_rejects_empty() {
        let req = CreateCommentRequest {
            content: "".to_string(),
        };
        assert!(req.validate().is_err());
    }

    #[test]
    fn test_update_comment_request_validates_content() {
        let req = UpdateCommentRequest {
            content: "Updated comment".to_string(),
        };
        assert!(req.validate().is_ok());
    }

    #[test]
    fn test_update_comment_request_rejects_empty() {
        let req = UpdateCommentRequest {
            content: "".to_string(),
        };
        assert!(req.validate().is_err());
    }
}
