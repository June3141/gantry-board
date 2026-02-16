use chrono::Utc;

use super::SyncEngine;
use crate::error::AppResult;
use crate::github::label_mapping;
use crate::models::github::{
    CreateIssueRequest, GitHubIssueMapping, GitHubLink, UpdateIssueRequest,
};
use crate::models::task::{Task, TaskStatus};
use crate::services::github_sync_service;

impl SyncEngine {
    /// Push a local task to GitHub as an issue. Creates or updates the issue.
    pub async fn push_task_to_github(
        &self,
        task: &Task,
        link: &GitHubLink,
    ) -> AppResult<GitHubIssueMapping> {
        let labels = label_mapping::build_labels_for_task(&task.status, &task.priority);
        let is_done = task.status == TaskStatus::Done;
        let existing = github_sync_service::get_mapping_by_task_id(&self.pool, task.id).await?;

        match existing {
            Some(mapping) => {
                let state = if is_done { "closed" } else { "open" };
                let req = UpdateIssueRequest {
                    title: Some(task.title.clone()),
                    body: task.description.clone(),
                    state: Some(state.to_string()),
                    labels: Some(labels),
                };
                self.github_client
                    .update_issue(
                        &link.repo_owner,
                        &link.repo_name,
                        mapping.github_issue_number as u64,
                        &req,
                    )
                    .await?;
                let now = Utc::now();
                github_sync_service::update_mapping_timestamps(
                    &self.pool,
                    mapping.id,
                    Some(now),
                    Some(now),
                )
                .await?;
                Ok(mapping)
            }
            None => {
                let req = CreateIssueRequest {
                    title: task.title.clone(),
                    body: task.description.clone(),
                    labels,
                };
                let issue = self
                    .github_client
                    .create_issue(&link.repo_owner, &link.repo_name, &req)
                    .await?;

                let mapping = github_sync_service::create_mapping(
                    &self.pool,
                    task.id,
                    link.id,
                    issue.number as i64,
                    Some(issue.id as i64),
                )
                .await?;

                // Close the issue if the task is done
                if is_done {
                    let close_req = UpdateIssueRequest {
                        title: None,
                        body: None,
                        state: Some("closed".to_string()),
                        labels: None,
                    };
                    self.github_client
                        .update_issue(&link.repo_owner, &link.repo_name, issue.number, &close_req)
                        .await?;
                }

                let now = Utc::now();
                github_sync_service::update_mapping_timestamps(
                    &self.pool,
                    mapping.id,
                    Some(now),
                    Some(now),
                )
                .await?;

                Ok(mapping)
            }
        }
    }

    /// Try to push a task to GitHub. No-op if project has no GitHub link.
    pub async fn try_push_task(&self, task: &Task) -> AppResult<()> {
        let link =
            crate::services::github_link_service::get_github_link(&self.pool, task.project_id)
                .await;
        match link {
            Ok(link) => {
                self.push_task_to_github(task, &link).await?;
                Ok(())
            }
            Err(crate::error::AppError::NotFound(_)) => Ok(()),
            Err(e) => Err(e),
        }
    }

    /// Ensure all gantry labels exist in the repository.
    pub async fn ensure_all_labels(&self, owner: &str, repo: &str) -> AppResult<()> {
        for def in label_mapping::all_label_definitions() {
            self.github_client
                .ensure_label(owner, repo, def.name, def.color)
                .await?;
        }
        Ok(())
    }
}
