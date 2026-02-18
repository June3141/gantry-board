use std::sync::Arc;
use std::time::Duration;

use chrono::{DateTime, Utc};
use moka::future::Cache;

use super::api::GitHubApi;
use crate::error::{AppError, AppResult};
use crate::models::github::{CreateIssueRequest, GitHubIssue, LinkedPr, UpdateIssueRequest};

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

    async fn list_prs_for_issue(
        &self,
        owner: &str,
        repo: &str,
        issue_number: u64,
    ) -> AppResult<Vec<LinkedPr>> {
        let per_page = 100usize;
        let mut page = 1u32;
        let mut all_events: Vec<serde_json::Value> = Vec::new();

        loop {
            let url = format!(
                "/repos/{owner}/{repo}/issues/{issue_number}/timeline?per_page={per_page}&page={page}"
            );
            let page_events: Vec<serde_json::Value> = self
                .client
                .get(&url, None::<&()>)
                .await
                .map_err(|e| AppError::Internal(format!("GitHub timeline API failed: {e}")))?;

            let fetched = page_events.len();
            all_events.extend(page_events);

            if fetched < per_page {
                break;
            }
            page += 1;
        }

        let mut prs = Vec::new();
        for event in all_events {
            if event.get("event").and_then(|v| v.as_str()) != Some("cross-referenced") {
                continue;
            }
            let Some(source) = event.get("source").and_then(|s| s.get("issue")) else {
                continue;
            };
            // Only include if this is a pull request
            if source.get("pull_request").is_none() {
                continue;
            }
            let pr_number = source.get("number").and_then(|v| v.as_u64()).unwrap_or(0);
            let title = source
                .get("title")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let html_url = source
                .get("html_url")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let state = source
                .get("state")
                .and_then(|v| v.as_str())
                .unwrap_or("open")
                .to_string();
            let is_merged = source
                .get("pull_request")
                .and_then(|pr| pr.get("merged_at"))
                .is_some_and(|v| !v.is_null());
            let author = source
                .get("user")
                .and_then(|u| u.get("login"))
                .and_then(|v| v.as_str())
                .map(String::from);

            if pr_number > 0 {
                prs.push(LinkedPr {
                    pr_number,
                    title,
                    url: html_url,
                    state,
                    is_merged,
                    author,
                });
            }
        }

        Ok(prs)
    }
}

/// Caching wrapper around any `GitHubApi` implementation.
/// Caches `list_issues` results with a configurable TTL (default 5 minutes).
pub struct CachedGitHubClient {
    inner: Arc<dyn GitHubApi>,
    issues_cache: Cache<String, Vec<GitHubIssue>>,
    label_cache: Cache<String, ()>,
}

impl CachedGitHubClient {
    pub fn new(inner: Arc<dyn GitHubApi>) -> Self {
        Self {
            inner,
            issues_cache: Cache::builder()
                .time_to_live(Duration::from_secs(300)) // 5 minutes
                .max_capacity(100)
                .build(),
            label_cache: Cache::builder()
                .time_to_live(Duration::from_secs(300))
                .max_capacity(500)
                .build(),
        }
    }

    /// Invalidate all cached data (e.g. after a webhook event).
    pub fn invalidate_all(&self) {
        self.issues_cache.invalidate_all();
        self.label_cache.invalidate_all();
    }
}

#[async_trait::async_trait]
impl GitHubApi for CachedGitHubClient {
    async fn check_connection(&self) -> AppResult<bool> {
        self.inner.check_connection().await
    }

    async fn list_issues(
        &self,
        owner: &str,
        repo: &str,
        since: Option<DateTime<Utc>>,
        state: &str,
    ) -> AppResult<Vec<GitHubIssue>> {
        let cache_key = format!(
            "{owner}/{repo}:{}:{state}",
            since.map_or("none".to_string(), |d| d.to_rfc3339())
        );

        if let Some(cached) = self.issues_cache.get(&cache_key).await {
            tracing::debug!(%cache_key, "GitHub issues cache hit");
            return Ok(cached);
        }

        let result = self.inner.list_issues(owner, repo, since, state).await?;
        self.issues_cache.insert(cache_key, result.clone()).await;
        Ok(result)
    }

    async fn create_issue(
        &self,
        owner: &str,
        repo: &str,
        req: &CreateIssueRequest,
    ) -> AppResult<GitHubIssue> {
        // Invalidate cache on write
        self.issues_cache.invalidate_all();
        self.inner.create_issue(owner, repo, req).await
    }

    async fn update_issue(
        &self,
        owner: &str,
        repo: &str,
        number: u64,
        req: &UpdateIssueRequest,
    ) -> AppResult<GitHubIssue> {
        self.issues_cache.invalidate_all();
        self.inner.update_issue(owner, repo, number, req).await
    }

    async fn ensure_label(
        &self,
        owner: &str,
        repo: &str,
        name: &str,
        color: &str,
    ) -> AppResult<()> {
        let cache_key = format!("{owner}/{repo}:{name}");
        if self.label_cache.get(&cache_key).await.is_some() {
            return Ok(());
        }
        self.inner.ensure_label(owner, repo, name, color).await?;
        self.label_cache.insert(cache_key, ()).await;
        Ok(())
    }

    async fn list_prs_for_issue(
        &self,
        owner: &str,
        repo: &str,
        issue_number: u64,
    ) -> AppResult<Vec<LinkedPr>> {
        // PR lists are not cached — they're infrequent and need fresh data
        self.inner
            .list_prs_for_issue(owner, repo, issue_number)
            .await
    }
}
