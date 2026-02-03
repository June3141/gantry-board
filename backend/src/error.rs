use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("not found: {0}")]
    NotFound(String),

    #[error("validation error: {0}")]
    Validation(String),

    #[error("conflict: {0}")]
    Conflict(String),

    /// Wrapper for all `sqlx::Error` values.
    ///
    /// We intentionally treat all SQLx errors the same at the HTTP boundary,
    /// returning `500 Internal Server Error`, except for constraint violations
    /// (unique/foreign key) which are detected and converted to `Conflict`.
    /// Higher layers using `fetch_optional` are responsible for translating
    /// "row not found" conditions into `AppError::NotFound`.
    #[error("database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error(transparent)]
    Internal(#[from] anyhow::Error),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, message) = match &self {
            AppError::NotFound(msg) => (StatusCode::NOT_FOUND, msg.clone()),
            AppError::Validation(msg) => (StatusCode::BAD_REQUEST, msg.clone()),
            AppError::Conflict(msg) => (StatusCode::CONFLICT, msg.clone()),
            AppError::Database(err) => {
                tracing::error!(%err, "database error");
                // Check for constraint violations
                let err_str = err.to_string();
                if err_str.contains("UNIQUE constraint failed") {
                    return AppError::Conflict("resource already exists".to_string())
                        .into_response();
                }
                if err_str.contains("FOREIGN KEY constraint failed") {
                    return AppError::Conflict("referenced resource does not exist".to_string())
                        .into_response();
                }
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "database error".to_string(),
                )
            }
            AppError::Internal(err) => {
                tracing::error!(%err, "internal server error");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "internal server error".to_string(),
                )
            }
        };

        let body = serde_json::json!({ "error": message });
        (status, axum::Json(body)).into_response()
    }
}

pub type AppResult<T> = Result<T, AppError>;
