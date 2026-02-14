use chrono::{DateTime, Utc};

use crate::error::AppResult;
use crate::models::github::{CreateIssueRequest, GitHubIssue, UpdateIssueRequest};

/// Abstraction over the GitHub API for testability.
#[async_trait::async_trait]
pub trait GitHubApi: Send + Sync {
    /// Check that the configured token can reach the GitHub API.
    async fn check_connection(&self) -> AppResult<bool>;

    /// List issues from a repository, optionally filtered by `since` timestamp.
    async fn list_issues(
        &self,
        owner: &str,
        repo: &str,
        since: Option<DateTime<Utc>>,
        state: &str,
    ) -> AppResult<Vec<GitHubIssue>>;

    /// Create a new issue in a repository.
    async fn create_issue(
        &self,
        owner: &str,
        repo: &str,
        req: &CreateIssueRequest,
    ) -> AppResult<GitHubIssue>;

    /// Update an existing issue in a repository.
    async fn update_issue(
        &self,
        owner: &str,
        repo: &str,
        number: u64,
        req: &UpdateIssueRequest,
    ) -> AppResult<GitHubIssue>;

    /// Ensure a label exists in a repository (create if missing).
    async fn ensure_label(&self, owner: &str, repo: &str, name: &str, color: &str)
        -> AppResult<()>;
}

/// No-op implementation for tests and when GitHub integration is disabled.
pub struct NoopGitHubClient;

#[async_trait::async_trait]
impl GitHubApi for NoopGitHubClient {
    async fn check_connection(&self) -> AppResult<bool> {
        Ok(false)
    }

    async fn list_issues(
        &self,
        _owner: &str,
        _repo: &str,
        _since: Option<DateTime<Utc>>,
        _state: &str,
    ) -> AppResult<Vec<GitHubIssue>> {
        Ok(vec![])
    }

    async fn create_issue(
        &self,
        _owner: &str,
        _repo: &str,
        req: &CreateIssueRequest,
    ) -> AppResult<GitHubIssue> {
        Ok(GitHubIssue {
            number: 1,
            id: 1,
            title: req.title.clone(),
            body: req.body.clone(),
            state: "open".to_string(),
            labels: req.labels.clone(),
            pull_request: false,
            updated_at: Utc::now(),
        })
    }

    async fn update_issue(
        &self,
        _owner: &str,
        _repo: &str,
        number: u64,
        req: &UpdateIssueRequest,
    ) -> AppResult<GitHubIssue> {
        Ok(GitHubIssue {
            number,
            id: 1,
            title: req.title.clone().unwrap_or_default(),
            body: req.body.clone(),
            state: req.state.clone().unwrap_or_else(|| "open".to_string()),
            labels: req.labels.clone().unwrap_or_default(),
            pull_request: false,
            updated_at: Utc::now(),
        })
    }

    async fn ensure_label(
        &self,
        _owner: &str,
        _repo: &str,
        _name: &str,
        _color: &str,
    ) -> AppResult<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_noop_client_check_connection_returns_false() {
        let client = NoopGitHubClient;
        let result = client.check_connection().await.unwrap();
        assert!(!result);
    }
}
