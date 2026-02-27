use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::Serialize;
use utoipa::ToSchema;

/// Machine-readable error codes for client-side handling.
#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ErrorCode {
    NotFound,
    ValidationFailed,
    Conflict,
    Unauthorized,
    Forbidden,
    InvalidCredentials,
    DatabaseError,
    GitError,
    InternalError,
}

/// Structured error detail.
#[derive(Debug, Serialize, ToSchema)]
pub struct ErrorDetail {
    pub code: ErrorCode,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
}

/// Standard API error response.
#[derive(Debug, Serialize, ToSchema)]
pub struct ErrorResponse {
    pub error: ErrorDetail,
    pub request_id: Option<String>,
    pub timestamp: String,
}

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("not found: {0}")]
    NotFound(String),

    #[error("validation error: {0}")]
    Validation(String),

    #[error("conflict: {0}")]
    Conflict(String),

    #[error("unauthorized")]
    Unauthorized,

    #[error("forbidden: {0}")]
    Forbidden(String),

    #[error("invalid credentials")]
    InvalidCredentials,

    #[error("internal error: {0}")]
    Internal(String),

    /// Wrapper for all `sqlx::Error` values.
    ///
    /// We intentionally treat all SQLx errors the same at the HTTP boundary,
    /// returning `500 Internal Server Error`, except for constraint violations
    /// (unique/foreign key) which are detected and converted to `Conflict`.
    /// Higher layers using `fetch_optional` are responsible for translating
    /// "row not found" conditions into `AppError::NotFound`.
    #[error("database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("git error: {0}")]
    Git(#[from] git2::Error),

    #[error(transparent)]
    Anyhow(#[from] anyhow::Error),
}

impl AppError {
    fn error_code(&self) -> ErrorCode {
        match self {
            AppError::NotFound(_) => ErrorCode::NotFound,
            AppError::Validation(_) => ErrorCode::ValidationFailed,
            AppError::Conflict(_) => ErrorCode::Conflict,
            AppError::Unauthorized => ErrorCode::Unauthorized,
            AppError::Forbidden(_) => ErrorCode::Forbidden,
            AppError::InvalidCredentials => ErrorCode::InvalidCredentials,
            AppError::Internal(_) | AppError::Anyhow(_) => ErrorCode::InternalError,
            AppError::Database(_) => ErrorCode::DatabaseError,
            AppError::Git(_) => ErrorCode::GitError,
        }
    }

    fn status_code(&self) -> StatusCode {
        match self {
            AppError::NotFound(_) => StatusCode::NOT_FOUND,
            AppError::Validation(_) => StatusCode::BAD_REQUEST,
            AppError::Conflict(_) => StatusCode::CONFLICT,
            AppError::Unauthorized | AppError::InvalidCredentials => StatusCode::UNAUTHORIZED,
            AppError::Forbidden(_) => StatusCode::FORBIDDEN,
            AppError::Internal(_) | AppError::Anyhow(_) | AppError::Database(_) => {
                StatusCode::INTERNAL_SERVER_ERROR
            }
            AppError::Git(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn user_message(&self) -> String {
        match self {
            AppError::NotFound(msg) => msg.clone(),
            AppError::Validation(msg) => msg.clone(),
            AppError::Conflict(msg) => msg.clone(),
            AppError::Unauthorized => "unauthorized".to_string(),
            AppError::Forbidden(msg) => msg.clone(),
            AppError::InvalidCredentials => "invalid credentials".to_string(),
            AppError::Internal(_) | AppError::Anyhow(_) => "internal server error".to_string(),
            AppError::Database(_) => "database error".to_string(),
            AppError::Git(_) => "git error".to_string(),
        }
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        // Handle special conversions first
        match &self {
            AppError::Database(err) => {
                tracing::debug!(%err, "database error details");
                tracing::error!("database error");
                let err_str = err.to_string();
                if err_str.contains("UNIQUE constraint failed") {
                    return AppError::Conflict("resource already exists".to_string())
                        .into_response();
                }
                if err_str.contains("FOREIGN KEY constraint failed") {
                    return AppError::Conflict("referenced resource does not exist".to_string())
                        .into_response();
                }
            }
            AppError::Git(err) => match err.code() {
                git2::ErrorCode::NotFound => {
                    tracing::info!(%err, "git not found");
                    return AppError::NotFound("resource not found".to_string()).into_response();
                }
                git2::ErrorCode::Exists => {
                    tracing::debug!(%err, "git conflict details");
                    return AppError::Conflict("git resource already exists".to_string())
                        .into_response();
                }
                git2::ErrorCode::InvalidSpec | git2::ErrorCode::Invalid => {
                    tracing::debug!(%err, "git validation error details");
                    return AppError::Validation("invalid git reference".to_string())
                        .into_response();
                }
                _ => {
                    tracing::debug!(%err, "git error details");
                    tracing::error!("git error");
                }
            },
            AppError::Internal(msg) => {
                tracing::debug!(msg, "internal server error details");
                tracing::error!("internal server error");
            }
            AppError::Anyhow(err) => {
                tracing::debug!(%err, "internal server error details");
                tracing::error!("internal server error");
            }
            _ => {}
        }

        let status = self.status_code();
        let error_code = self.error_code();
        let message = self.user_message();

        // Record error metric
        let error_code_label = serde_json::to_string(&error_code)
            .unwrap_or_else(|_| "UNKNOWN".to_string())
            .trim_matches('"')
            .to_string();
        metrics::counter!(
            crate::observability::metric::ERRORS_TOTAL,
            "error_code" => error_code_label
        )
        .increment(1);

        let body = ErrorResponse {
            error: ErrorDetail {
                code: error_code,
                message,
                details: None,
            },
            // request_id is injected by the `inject_request_id_into_errors` middleware
            // after this IntoResponse impl runs. Left as None here.
            request_id: None,
            timestamp: chrono::Utc::now().to_rfc3339(),
        };

        (status, axum::Json(body)).into_response()
    }
}

pub type AppResult<T> = Result<T, AppError>;

/// Middleware that normalizes all error responses to the standard structure
/// and injects request_id from the x-request-id header.
pub async fn inject_request_id_into_errors(
    request: axum::extract::Request,
    next: axum::middleware::Next,
) -> Response {
    let request_id = request
        .headers()
        .get("x-request-id")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    let response = next.run(request).await;

    if !response.status().is_client_error() && !response.status().is_server_error() {
        return response;
    }

    let status = response.status();
    let (parts, body) = response.into_parts();

    let Ok(bytes) = axum::body::to_bytes(body, usize::MAX).await else {
        return Response::from_parts(parts, axum::body::Body::empty());
    };

    // Try to parse as JSON
    if let Ok(mut json) = serde_json::from_slice::<serde_json::Value>(bytes.as_ref()) {
        // Already in our standard format — just inject request_id
        if json.get("error").is_some()
            && json["error"].get("code").is_some()
            && json.get("request_id").is_some()
        {
            if let Some(rid) = &request_id {
                json["request_id"] = serde_json::Value::String(rid.clone());
            }
            let new_body = serde_json::to_vec(&json).unwrap_or_else(|_| bytes.to_vec());
            return Response::from_parts(parts, axum::body::Body::from(new_body));
        }
    }

    // Non-standard error response (e.g. axum's JsonRejection) — wrap it
    let message = String::from_utf8_lossy(&bytes).to_string();
    let error_code = error_code_from_status(status);

    let body = ErrorResponse {
        error: ErrorDetail {
            code: error_code,
            message,
            details: None,
        },
        request_id,
        timestamp: chrono::Utc::now().to_rfc3339(),
    };

    let new_body = serde_json::to_vec(&body).unwrap_or_else(|_| bytes.to_vec());
    Response::from_parts(parts, axum::body::Body::from(new_body))
}

fn error_code_from_status(status: StatusCode) -> ErrorCode {
    match status {
        StatusCode::NOT_FOUND => ErrorCode::NotFound,
        StatusCode::BAD_REQUEST | StatusCode::UNPROCESSABLE_ENTITY => ErrorCode::ValidationFailed,
        StatusCode::CONFLICT => ErrorCode::Conflict,
        StatusCode::UNAUTHORIZED => ErrorCode::Unauthorized,
        StatusCode::FORBIDDEN => ErrorCode::Forbidden,
        _ => ErrorCode::InternalError,
    }
}
