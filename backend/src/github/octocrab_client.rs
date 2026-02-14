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

    fn convert_issue(issue: octocrab::models::issues::Issue) -> GitHubIssue {
        GitHubIssue {
            number: issue.number,
            id: issue.id.into_inner(),
            title: issue.title,
            body: issue.body,
            state: match issue.state {
                octocrab::models::IssueState::Open => "open".to_string(),
                _ => "closed".to_string(),
            },
            labels: issue.labels.iter().map(|l| l.name.clone()).collect(),
            pull_request: issue.pull_request.is_some(),
            updated_at: issue.updated_at,
        }
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
        owner: &str,
        repo: &str,
        since: Option<DateTime<Utc>>,
        state: &str,
    ) -> AppResult<Vec<GitHubIssue>> {
        let param_state = match state {
            "open" => octocrab::params::State::Open,
            "closed" => octocrab::params::State::Closed,
            _ => octocrab::params::State::All,
        };

        let issues_handler = self.client.issues(owner, repo);
        let mut builder = issues_handler.list().state(param_state).per_page(100);

        if let Some(dt) = since {
            builder = builder.since(dt);
        }

        let page = builder
            .send()
            .await
            .map_err(|e| AppError::Internal(format!("GitHub list_issues failed: {e}")))?;

        Ok(page.items.into_iter().map(Self::convert_issue).collect())
    }

    async fn create_issue(
        &self,
        owner: &str,
        repo: &str,
        req: &CreateIssueRequest,
    ) -> AppResult<GitHubIssue> {
        let issues_handler = self.client.issues(owner, repo);
        let mut builder = issues_handler.create(&req.title);

        if let Some(body) = &req.body {
            builder = builder.body(body);
        }

        if !req.labels.is_empty() {
            builder = builder.labels(Some(req.labels.clone()));
        }

        let issue = builder
            .send()
            .await
            .map_err(|e| AppError::Internal(format!("GitHub create_issue failed: {e}")))?;

        Ok(Self::convert_issue(issue))
    }

    async fn update_issue(
        &self,
        owner: &str,
        repo: &str,
        number: u64,
        req: &UpdateIssueRequest,
    ) -> AppResult<GitHubIssue> {
        let issues_handler = self.client.issues(owner, repo);
        let mut builder = issues_handler.update(number);

        if let Some(title) = &req.title {
            builder = builder.title(title);
        }
        if let Some(body) = &req.body {
            builder = builder.body(body);
        }
        if let Some(state_str) = &req.state {
            let issue_state = match state_str.as_str() {
                "closed" => octocrab::models::IssueState::Closed,
                _ => octocrab::models::IssueState::Open,
            };
            builder = builder.state(issue_state);
        }
        if let Some(labels) = &req.labels {
            builder = builder.labels(labels);
        }

        let issue = builder
            .send()
            .await
            .map_err(|e| AppError::Internal(format!("GitHub update_issue failed: {e}")))?;

        Ok(Self::convert_issue(issue))
    }

    async fn ensure_label(
        &self,
        owner: &str,
        repo: &str,
        name: &str,
        color: &str,
    ) -> AppResult<()> {
        let issues_handler = self.client.issues(owner, repo);
        match issues_handler.get_label(name).await {
            Ok(_) => Ok(()),
            Err(_) => {
                self.client
                    .issues(owner, repo)
                    .create_label(name, color, "")
                    .await
                    .map_err(|e| AppError::Internal(format!("GitHub ensure_label failed: {e}")))?;
                Ok(())
            }
        }
    }
}
