use serde::Serialize;
use utoipa::ToSchema;

use crate::error::{AppError, AppResult};

const DEFAULT_LIMIT: i64 = 50;
const MAX_LIMIT: i64 = 100;

pub fn default_limit() -> i64 {
    DEFAULT_LIMIT
}

pub fn validate(limit: i64, offset: i64) -> AppResult<()> {
    if !(1..=MAX_LIMIT).contains(&limit) {
        return Err(AppError::Validation(format!(
            "limit must be between 1 and {MAX_LIMIT}"
        )));
    }
    if offset < 0 {
        return Err(AppError::Validation(
            "offset must be non-negative".to_string(),
        ));
    }
    Ok(())
}

#[derive(Debug, Serialize, ToSchema)]
pub struct PaginatedResponse<T: Serialize> {
    pub data: Vec<T>,
    pub total: i64,
    pub limit: i64,
    pub offset: i64,
}
