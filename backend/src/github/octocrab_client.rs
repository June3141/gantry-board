use super::api::GitHubApi;
use crate::error::{AppError, AppResult};

pub struct OctocrabClient {
    client: octocrab::Octocrab,
}

impl OctocrabClient {
    pub fn new(token: &str) -> AppResult<Self> {
        let client = octocrab::Octocrab::builder()
            .personal_token(token.to_string())
            .build()
            .map_err(|e| AppError::Internal(e.to_string()))?;
        Ok(Self { client })
    }
}

#[async_trait::async_trait]
impl GitHubApi for OctocrabClient {
    async fn check_connection(&self) -> AppResult<bool> {
        match self.client.current().user().await {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    }
}
