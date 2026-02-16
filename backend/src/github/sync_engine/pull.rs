use super::SyncEngine;
use crate::error::AppResult;
use crate::models::github::GitHubLink;
use crate::services::github_sync_service;

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
}
