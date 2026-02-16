use super::SyncEngine;
use crate::error::AppResult;
use crate::models::github::{GitHubIssue, GitHubLink};
use crate::models::task::{CreateTaskRequest, Task, TaskPriority, TaskStatus, UpdateTaskRequest};
use crate::services::{github_pr_service, github_sync_service, task_service};

use crate::github::label_mapping;

impl SyncEngine {
    /// Pull issues from GitHub and create/update local tasks.
    /// Returns (created_count, updated_count).
    pub async fn pull_issues_from_github(&self, link: &GitHubLink) -> AppResult<(u32, u32)> {
        let issues = self
            .github_client
            .list_issues(
                &link.repo_owner,
                &link.repo_name,
                link.last_synced_at,
                "all",
            )
            .await?;

        let mut created = 0u32;
        let mut updated = 0u32;

        for issue in issues {
            if issue.pull_request {
                continue;
            }

            let existing = github_sync_service::get_mapping_by_issue_number(
                &self.pool,
                link.id,
                issue.number as i64,
            )
            .await?;

            match existing {
                Some(mapping) => {
                    // Last-write-wins: skip if local is newer
                    if let Some(local_time) = mapping.last_local_update {
                        if local_time >= issue.updated_at {
                            continue;
                        }
                    }
                    self.update_task_from_issue(&issue, mapping.task_id).await?;
                    github_sync_service::update_mapping_timestamps(
                        &self.pool,
                        mapping.id,
                        None,
                        Some(issue.updated_at),
                    )
                    .await?;
                    updated += 1;
                }
                None => {
                    let task = self.create_task_from_issue(&issue, link.project_id).await?;
                    github_sync_service::create_mapping(
                        &self.pool,
                        task.id,
                        link.id,
                        issue.number as i64,
                        Some(issue.id as i64),
                    )
                    .await?;
                    created += 1;
                }
            }
        }

        Ok((created, updated))
    }

    /// Detect pull requests linked to mapped issues and save them to DB.
    /// Soft failure: logs a warning and continues on error.
    pub async fn detect_pull_requests(&self, link: &GitHubLink) {
        let mappings = match github_sync_service::list_mappings_by_link(&self.pool, link.id).await {
            Ok(m) => m,
            Err(e) => {
                tracing::warn!(error = %e, "failed to list mappings for PR detection");
                return;
            }
        };

        for mapping in &mappings {
            let prs = match self
                .github_client
                .list_prs_for_issue(
                    &link.repo_owner,
                    &link.repo_name,
                    mapping.github_issue_number as u64,
                )
                .await
            {
                Ok(prs) => prs,
                Err(e) => {
                    tracing::warn!(
                        issue_number = mapping.github_issue_number,
                        error = %e,
                        "failed to list PRs for issue, skipping"
                    );
                    continue;
                }
            };

            for pr in &prs {
                if let Err(e) =
                    github_pr_service::upsert_pr(&self.pool, link.id, mapping.task_id, pr).await
                {
                    tracing::warn!(
                        pr_number = pr.pr_number,
                        error = %e,
                        "failed to upsert PR, skipping"
                    );
                }
            }
        }
    }

    fn extract_status(issue: &GitHubIssue) -> TaskStatus {
        if issue.state == "closed" {
            return TaskStatus::Done;
        }
        label_mapping::extract_status_from_labels(&issue.labels).unwrap_or(TaskStatus::Backlog)
    }

    fn extract_priority(issue: &GitHubIssue) -> TaskPriority {
        label_mapping::extract_priority_from_labels(&issue.labels).unwrap_or(TaskPriority::Medium)
    }

    async fn create_task_from_issue(
        &self,
        issue: &GitHubIssue,
        project_id: uuid::Uuid,
    ) -> AppResult<Task> {
        let req = CreateTaskRequest {
            project_id,
            title: issue.title.clone(),
            description: issue.body.clone(),
            status: Some(Self::extract_status(issue)),
            priority: Some(Self::extract_priority(issue)),
            parent_id: None,
            assigned_to: None,
        };
        task_service::create_task(&self.pool, &req).await
    }

    async fn update_task_from_issue(
        &self,
        issue: &GitHubIssue,
        task_id: uuid::Uuid,
    ) -> AppResult<Task> {
        let req = UpdateTaskRequest {
            title: Some(issue.title.clone()),
            description: issue.body.clone(),
            status: Some(Self::extract_status(issue)),
            priority: Some(Self::extract_priority(issue)),
            parent_id: None,
            assigned_to: None,
            position: None,
        };
        task_service::update_task(&self.pool, task_id, &req).await
    }
}
