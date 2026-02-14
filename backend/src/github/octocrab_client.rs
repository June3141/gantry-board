use chrono::{DateTime, Utc};

use super::api::GitHubApi;
use crate::error::{AppError, AppResult};
use crate::models::github::{CreateIssueRequest, GitHubIssue, UpdateIssueRequest};

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

    async fn list_issues(
        &self,
        _owner: &str,
        _repo: &str,
        _since: Option<DateTime<Utc>>,
        _state: &str,
    ) -> AppResult<Vec<GitHubIssue>> {
        todo!()
    }

    async fn create_issue(
        &self,
        _owner: &str,
        _repo: &str,
        _req: &CreateIssueRequest,
    ) -> AppResult<GitHubIssue> {
        todo!()
    }

    async fn update_issue(
        &self,
        _owner: &str,
        _repo: &str,
        _number: u64,
        _req: &UpdateIssueRequest,
    ) -> AppResult<GitHubIssue> {
        todo!()
    }

    async fn ensure_label(
        &self,
        _owner: &str,
        _repo: &str,
        _name: &str,
        _color: &str,
    ) -> AppResult<()> {
        todo!()
    }
}
