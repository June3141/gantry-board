use crate::error::AppResult;

/// Abstraction over the GitHub API for testability.
#[async_trait::async_trait]
pub trait GitHubApi: Send + Sync {
    /// Check that the configured token can reach the GitHub API.
    async fn check_connection(&self) -> AppResult<bool>;
}

/// No-op implementation for tests and when GitHub integration is disabled.
pub struct NoopGitHubClient;

#[async_trait::async_trait]
impl GitHubApi for NoopGitHubClient {
    async fn check_connection(&self) -> AppResult<bool> {
        Ok(false)
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
