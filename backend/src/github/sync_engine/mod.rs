mod pull;
mod push;

use sqlx::SqlitePool;
use std::sync::Arc;

use super::api::GitHubApi;
use super::label_mapping;
use crate::error::AppResult;
use crate::models::github::{GitHubIssue, GitHubLink, SyncResult};
use crate::models::task::{CreateTaskRequest, Task, TaskPriority, TaskStatus, UpdateTaskRequest};
use crate::services::{github_link_service, github_pr_service, github_sync_service, task_service};

pub struct SyncEngine {
    pub(crate) github_client: Arc<dyn GitHubApi>,
    pub(crate) pool: SqlitePool,
}

impl SyncEngine {
    pub fn new(github_client: Arc<dyn GitHubApi>, pool: SqlitePool) -> Self {
        Self {
            github_client,
            pool,
        }
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

    /// Run a full sync for one project: ensure labels, push, pull, detect PRs, update last_synced.
    pub async fn sync_project(&self, link: &GitHubLink) -> AppResult<SyncResult> {
        self.ensure_all_labels(&link.repo_owner, &link.repo_name)
            .await?;

        // Push all unmapped local tasks
        let tasks = task_service::list_tasks(&self.pool, link.project_id).await?;
        let mut pushed = 0u32;
        for task in &tasks {
            let mapping = github_sync_service::get_mapping_by_task_id(&self.pool, task.id).await?;
            if mapping.is_none() {
                self.push_task_to_github(task, link).await?;
                pushed += 1;
            }
        }

        // Pull from GitHub
        let (pulled_created, pulled_updated) = self.pull_issues_from_github(link).await?;

        // Detect linked PRs (soft failure)
        self.detect_pull_requests(link).await;

        // Update last_synced_at
        github_link_service::update_last_synced(&self.pool, link.project_id).await?;

        Ok(SyncResult {
            project_id: link.project_id,
            pushed,
            pulled: pulled_created + pulled_updated,
        })
    }

    /// Sync all projects that have sync enabled.
    /// Continues with remaining projects if one fails.
    pub async fn sync_all(&self) -> AppResult<Vec<SyncResult>> {
        let links = github_link_service::list_sync_enabled(&self.pool).await?;
        let mut results = Vec::new();
        for link in &links {
            match self.sync_project(link).await {
                Ok(result) => results.push(result),
                Err(e) => {
                    tracing::warn!(
                        project_id = %link.project_id,
                        error = %e,
                        "sync failed for project, continuing with next"
                    );
                }
            }
        }
        Ok(results)
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::github::{CreateIssueRequest, GitHubIssue, LinkedPr, UpdateIssueRequest};
    use crate::models::task::{Task, TaskPriority, TaskStatus};
    use chrono::{DateTime, Utc};
    use sqlx::sqlite::SqlitePoolOptions;
    use std::collections::HashMap;
    use std::sync::Mutex;
    use uuid::Uuid;

    /// Mock implementation of GitHubApi that records calls.
    struct MockGitHubApi {
        created_issues: Mutex<Vec<CreateIssueRequest>>,
        updated_issues: Mutex<Vec<(u64, UpdateIssueRequest)>>,
        ensured_labels: Mutex<Vec<(String, String)>>,
        /// Issues returned by list_issues.
        issues: Mutex<Vec<GitHubIssue>>,
        /// Next issue number to assign.
        next_number: Mutex<u64>,
        /// PRs returned by list_prs_for_issue, keyed by issue number.
        linked_prs: Mutex<HashMap<u64, Vec<LinkedPr>>>,
    }

    impl MockGitHubApi {
        fn new() -> Self {
            Self {
                created_issues: Mutex::new(vec![]),
                updated_issues: Mutex::new(vec![]),
                ensured_labels: Mutex::new(vec![]),
                issues: Mutex::new(vec![]),
                next_number: Mutex::new(1),
                linked_prs: Mutex::new(HashMap::new()),
            }
        }

        fn with_issues(issues: Vec<GitHubIssue>) -> Self {
            Self {
                issues: Mutex::new(issues),
                ..Self::new()
            }
        }
    }

    #[async_trait::async_trait]
    impl GitHubApi for MockGitHubApi {
        async fn check_connection(&self) -> AppResult<bool> {
            Ok(true)
        }

        async fn list_issues(
            &self,
            _owner: &str,
            _repo: &str,
            _since: Option<DateTime<Utc>>,
            _state: &str,
        ) -> AppResult<Vec<GitHubIssue>> {
            Ok(self.issues.lock().unwrap().clone())
        }

        async fn create_issue(
            &self,
            _owner: &str,
            _repo: &str,
            req: &CreateIssueRequest,
        ) -> AppResult<GitHubIssue> {
            let mut num = self.next_number.lock().unwrap();
            let number = *num;
            *num += 1;
            self.created_issues
                .lock()
                .unwrap()
                .push(CreateIssueRequest {
                    title: req.title.clone(),
                    body: req.body.clone(),
                    labels: req.labels.clone(),
                });
            Ok(GitHubIssue {
                number,
                id: number * 1000,
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
            self.updated_issues.lock().unwrap().push((
                number,
                UpdateIssueRequest {
                    title: req.title.clone(),
                    body: req.body.clone(),
                    state: req.state.clone(),
                    labels: req.labels.clone(),
                },
            ));
            Ok(GitHubIssue {
                number,
                id: number * 1000,
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
            name: &str,
            color: &str,
        ) -> AppResult<()> {
            self.ensured_labels
                .lock()
                .unwrap()
                .push((name.to_string(), color.to_string()));
            Ok(())
        }

        async fn list_prs_for_issue(
            &self,
            _owner: &str,
            _repo: &str,
            issue_number: u64,
        ) -> AppResult<Vec<LinkedPr>> {
            Ok(self
                .linked_prs
                .lock()
                .unwrap()
                .get(&issue_number)
                .cloned()
                .unwrap_or_default())
        }
    }

    async fn setup_db() -> SqlitePool {
        let pool = SqlitePoolOptions::new()
            .connect("sqlite::memory:")
            .await
            .unwrap();
        sqlx::migrate!("./migrations").run(&pool).await.unwrap();
        pool
    }

    async fn create_project(pool: &SqlitePool) -> Uuid {
        let id = Uuid::new_v4();
        sqlx::query("INSERT INTO projects (id, name) VALUES ($1, $2)")
            .bind(id.to_string())
            .bind("test-project")
            .execute(pool)
            .await
            .unwrap();
        id
    }

    fn make_link(project_id: Uuid) -> GitHubLink {
        GitHubLink {
            id: Uuid::new_v4(),
            project_id,
            repo_owner: "owner".to_string(),
            repo_name: "repo".to_string(),
            sync_enabled: true,
            last_synced_at: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    fn make_task(project_id: Uuid, status: TaskStatus, priority: TaskPriority) -> Task {
        Task {
            id: Uuid::new_v4(),
            project_id,
            title: "Test Task".to_string(),
            description: Some("Description".to_string()),
            status,
            priority,
            parent_id: None,
            assigned_to: None,
            position: 0,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    async fn insert_link(pool: &SqlitePool, link: &GitHubLink) {
        sqlx::query(
            "INSERT INTO github_links (id, project_id, repo_owner, repo_name) VALUES ($1, $2, $3, $4)",
        )
        .bind(link.id.to_string())
        .bind(link.project_id.to_string())
        .bind(&link.repo_owner)
        .bind(&link.repo_name)
        .execute(pool)
        .await
        .unwrap();
    }

    fn status_to_db(status: &TaskStatus) -> &'static str {
        match status {
            TaskStatus::Backlog => "backlog",
            TaskStatus::Todo => "todo",
            TaskStatus::InProgress => "in_progress",
            TaskStatus::InReview => "in_review",
            TaskStatus::Done => "done",
        }
    }

    fn priority_to_db(priority: &TaskPriority) -> &'static str {
        match priority {
            TaskPriority::Low => "low",
            TaskPriority::Medium => "medium",
            TaskPriority::High => "high",
            TaskPriority::Urgent => "urgent",
        }
    }

    async fn insert_task(pool: &SqlitePool, task: &Task) {
        sqlx::query(
            "INSERT INTO tasks (id, project_id, title, description, status, priority, position) VALUES ($1, $2, $3, $4, $5, $6, $7)",
        )
        .bind(task.id.to_string())
        .bind(task.project_id.to_string())
        .bind(&task.title)
        .bind(&task.description)
        .bind(status_to_db(&task.status))
        .bind(priority_to_db(&task.priority))
        .bind(task.position)
        .execute(pool)
        .await
        .unwrap();
    }

    // --- push_task_to_github tests ---

    #[tokio::test]
    async fn push_creates_new_issue_when_no_mapping_exists() {
        let pool = setup_db().await;
        let project_id = create_project(&pool).await;
        let link = make_link(project_id);
        insert_link(&pool, &link).await;
        let task = make_task(project_id, TaskStatus::Todo, TaskPriority::High);
        insert_task(&pool, &task).await;

        let mock = Arc::new(MockGitHubApi::new());
        let engine = SyncEngine::new(Arc::clone(&mock) as Arc<dyn GitHubApi>, pool.clone());

        let mapping = engine.push_task_to_github(&task, &link).await.unwrap();

        assert_eq!(mapping.task_id, task.id);
        assert_eq!(mapping.github_issue_number, 1);
        assert!(mapping.github_issue_id.is_some());

        let created = mock.created_issues.lock().unwrap();
        assert_eq!(created.len(), 1);
        assert_eq!(created[0].title, "Test Task");
        assert!(created[0].labels.contains(&"status:todo".to_string()));
        assert!(created[0].labels.contains(&"priority:high".to_string()));
    }

    #[tokio::test]
    async fn push_updates_existing_issue_when_mapping_exists() {
        let pool = setup_db().await;
        let project_id = create_project(&pool).await;
        let link = make_link(project_id);
        insert_link(&pool, &link).await;
        let task = make_task(project_id, TaskStatus::InProgress, TaskPriority::Medium);
        insert_task(&pool, &task).await;

        // Create an existing mapping
        crate::services::github_sync_service::create_mapping(
            &pool,
            task.id,
            link.id,
            42,
            Some(42000),
        )
        .await
        .unwrap();

        let mock = Arc::new(MockGitHubApi::new());
        let engine = SyncEngine::new(Arc::clone(&mock) as Arc<dyn GitHubApi>, pool.clone());

        let mapping = engine.push_task_to_github(&task, &link).await.unwrap();

        assert_eq!(mapping.github_issue_number, 42);

        let created = mock.created_issues.lock().unwrap();
        assert!(created.is_empty(), "Should not create a new issue");

        let updated = mock.updated_issues.lock().unwrap();
        assert_eq!(updated.len(), 1);
        assert_eq!(updated[0].0, 42);
        let labels = updated[0].1.labels.as_ref().unwrap();
        assert!(labels.contains(&"status:in_progress".to_string()));
        assert!(labels.contains(&"priority:medium".to_string()));
    }

    #[tokio::test]
    async fn push_sets_state_closed_for_done_tasks() {
        let pool = setup_db().await;
        let project_id = create_project(&pool).await;
        let link = make_link(project_id);
        insert_link(&pool, &link).await;
        let task = make_task(project_id, TaskStatus::Done, TaskPriority::Low);
        insert_task(&pool, &task).await;

        let mock = Arc::new(MockGitHubApi::new());
        let engine = SyncEngine::new(Arc::clone(&mock) as Arc<dyn GitHubApi>, pool.clone());

        engine.push_task_to_github(&task, &link).await.unwrap();

        let created = mock.created_issues.lock().unwrap();
        assert_eq!(created.len(), 1);
        // New issue is created as open; then an update would close it
        // But for simplicity the create sets labels — the close happens via update if mapping exists.
        // For new issues that are Done, the engine should create then update to closed.
        // Let's check if the issue was updated to closed too.
        drop(created);
        // Actually — for a new Done task, the engine should create the issue first then
        // close it immediately. Let's check the updated_issues.
        let updated = mock.updated_issues.lock().unwrap();
        assert_eq!(updated.len(), 1);
        assert_eq!(updated[0].1.state.as_deref(), Some("closed"));
    }

    #[tokio::test]
    async fn push_reopens_issue_when_status_changes_from_done() {
        let pool = setup_db().await;
        let project_id = create_project(&pool).await;
        let link = make_link(project_id);
        insert_link(&pool, &link).await;
        let task = make_task(project_id, TaskStatus::InProgress, TaskPriority::Medium);
        insert_task(&pool, &task).await;

        // Create mapping as if issue was previously closed
        crate::services::github_sync_service::create_mapping(
            &pool,
            task.id,
            link.id,
            50,
            Some(50000),
        )
        .await
        .unwrap();

        let mock = Arc::new(MockGitHubApi::new());
        let engine = SyncEngine::new(Arc::clone(&mock) as Arc<dyn GitHubApi>, pool.clone());

        engine.push_task_to_github(&task, &link).await.unwrap();

        let updated = mock.updated_issues.lock().unwrap();
        assert_eq!(updated.len(), 1);
        assert_eq!(updated[0].1.state.as_deref(), Some("open"));
    }

    #[tokio::test]
    async fn ensure_all_labels_calls_ensure_label_for_all_definitions() {
        let pool = setup_db().await;
        let mock = Arc::new(MockGitHubApi::new());
        let engine = SyncEngine::new(Arc::clone(&mock) as Arc<dyn GitHubApi>, pool);

        engine.ensure_all_labels("owner", "repo").await.unwrap();

        let labels = mock.ensured_labels.lock().unwrap();
        // 5 status + 4 priority = 9
        assert_eq!(labels.len(), 9);
        assert!(labels.iter().any(|(n, _)| n == "status:todo"));
        assert!(labels.iter().any(|(n, _)| n == "priority:urgent"));
    }

    // --- pull_issues_from_github tests ---

    fn make_github_issue(number: u64, title: &str, state: &str, labels: Vec<&str>) -> GitHubIssue {
        GitHubIssue {
            number,
            id: number * 1000,
            title: title.to_string(),
            body: Some("Issue body".to_string()),
            state: state.to_string(),
            labels: labels.into_iter().map(String::from).collect(),
            pull_request: false,
            updated_at: Utc::now(),
        }
    }

    #[tokio::test]
    async fn pull_creates_task_for_new_issue() {
        let pool = setup_db().await;
        let project_id = create_project(&pool).await;
        let link = make_link(project_id);
        insert_link(&pool, &link).await;

        let issue = make_github_issue(
            10,
            "New Issue",
            "open",
            vec!["status:todo", "priority:high"],
        );
        let mock = Arc::new(MockGitHubApi::with_issues(vec![issue]));
        let engine = SyncEngine::new(Arc::clone(&mock) as Arc<dyn GitHubApi>, pool.clone());

        let (created, updated) = engine.pull_issues_from_github(&link).await.unwrap();
        assert_eq!(created, 1);
        assert_eq!(updated, 0);

        // Verify mapping was created
        let mapping = github_sync_service::get_mapping_by_issue_number(&pool, link.id, 10)
            .await
            .unwrap();
        assert!(mapping.is_some());
    }

    #[tokio::test]
    async fn pull_updates_task_when_remote_is_newer() {
        let pool = setup_db().await;
        let project_id = create_project(&pool).await;
        let link = make_link(project_id);
        insert_link(&pool, &link).await;
        let task = make_task(project_id, TaskStatus::Todo, TaskPriority::Medium);
        insert_task(&pool, &task).await;

        // Create mapping with old local timestamp
        let mapping = github_sync_service::create_mapping(&pool, task.id, link.id, 20, Some(20000))
            .await
            .unwrap();
        let old_time = Utc::now() - chrono::Duration::hours(1);
        github_sync_service::update_mapping_timestamps(&pool, mapping.id, Some(old_time), None)
            .await
            .unwrap();

        // GitHub issue is newer
        let issue = make_github_issue(
            20,
            "Updated Title",
            "open",
            vec!["status:in_progress", "priority:urgent"],
        );
        let mock = Arc::new(MockGitHubApi::with_issues(vec![issue]));
        let engine = SyncEngine::new(Arc::clone(&mock) as Arc<dyn GitHubApi>, pool.clone());

        let (created, updated) = engine.pull_issues_from_github(&link).await.unwrap();
        assert_eq!(created, 0);
        assert_eq!(updated, 1);
    }

    #[tokio::test]
    async fn pull_skips_when_local_is_newer() {
        let pool = setup_db().await;
        let project_id = create_project(&pool).await;
        let link = make_link(project_id);
        insert_link(&pool, &link).await;
        let task = make_task(project_id, TaskStatus::InProgress, TaskPriority::High);
        insert_task(&pool, &task).await;

        // Create mapping with recent local timestamp
        let mapping = github_sync_service::create_mapping(&pool, task.id, link.id, 30, Some(30000))
            .await
            .unwrap();
        let recent = Utc::now() + chrono::Duration::hours(1);
        github_sync_service::update_mapping_timestamps(&pool, mapping.id, Some(recent), None)
            .await
            .unwrap();

        // GitHub issue is older
        let mut issue = make_github_issue(30, "Old Title", "open", vec!["status:todo"]);
        issue.updated_at = Utc::now() - chrono::Duration::hours(2);
        let mock = Arc::new(MockGitHubApi::with_issues(vec![issue]));
        let engine = SyncEngine::new(Arc::clone(&mock) as Arc<dyn GitHubApi>, pool.clone());

        let (created, updated) = engine.pull_issues_from_github(&link).await.unwrap();
        assert_eq!(created, 0);
        assert_eq!(updated, 0);
    }

    #[tokio::test]
    async fn pull_skips_pull_requests() {
        let pool = setup_db().await;
        let project_id = create_project(&pool).await;
        let link = make_link(project_id);
        insert_link(&pool, &link).await;

        let mut pr = make_github_issue(40, "A PR", "open", vec![]);
        pr.pull_request = true;
        let mock = Arc::new(MockGitHubApi::with_issues(vec![pr]));
        let engine = SyncEngine::new(Arc::clone(&mock) as Arc<dyn GitHubApi>, pool.clone());

        let (created, updated) = engine.pull_issues_from_github(&link).await.unwrap();
        assert_eq!(created, 0);
        assert_eq!(updated, 0);
    }

    #[tokio::test]
    async fn pull_marks_task_done_when_issue_is_closed() {
        let pool = setup_db().await;
        let project_id = create_project(&pool).await;
        let link = make_link(project_id);
        insert_link(&pool, &link).await;

        let issue = make_github_issue(
            50,
            "Closed Issue",
            "closed",
            vec!["status:done", "priority:low"],
        );
        let mock = Arc::new(MockGitHubApi::with_issues(vec![issue]));
        let engine = SyncEngine::new(Arc::clone(&mock) as Arc<dyn GitHubApi>, pool.clone());

        let (created, _updated) = engine.pull_issues_from_github(&link).await.unwrap();
        assert_eq!(created, 1);

        // Verify task was created with Done status
        let mapping = github_sync_service::get_mapping_by_issue_number(&pool, link.id, 50)
            .await
            .unwrap()
            .unwrap();
        let task: (String,) = sqlx::query_as("SELECT status FROM tasks WHERE id = $1")
            .bind(mapping.task_id.to_string())
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(task.0, "done");
    }

    // --- sync_project tests ---

    #[tokio::test]
    async fn sync_project_returns_combined_result() {
        let pool = setup_db().await;
        let project_id = create_project(&pool).await;
        let link = make_link(project_id);
        insert_link(&pool, &link).await;

        // One local task to push
        let task = make_task(project_id, TaskStatus::Todo, TaskPriority::Medium);
        insert_task(&pool, &task).await;

        // One remote issue to pull
        let issue = make_github_issue(
            60,
            "Remote Issue",
            "open",
            vec!["status:backlog", "priority:low"],
        );
        let mock = Arc::new(MockGitHubApi::with_issues(vec![issue]));
        let engine = SyncEngine::new(Arc::clone(&mock) as Arc<dyn GitHubApi>, pool.clone());

        let result = engine.sync_project(&link).await.unwrap();
        assert_eq!(result.project_id, project_id);
        assert!(result.pushed >= 1);
        assert!(result.pulled >= 1);
    }

    // --- try_push_task tests ---

    #[tokio::test]
    async fn try_push_task_pushes_when_link_exists() {
        let pool = setup_db().await;
        let project_id = create_project(&pool).await;
        let link = make_link(project_id);
        insert_link(&pool, &link).await;
        let task = make_task(project_id, TaskStatus::Todo, TaskPriority::High);
        insert_task(&pool, &task).await;

        let mock = Arc::new(MockGitHubApi::new());
        let engine = SyncEngine::new(Arc::clone(&mock) as Arc<dyn GitHubApi>, pool.clone());

        engine.try_push_task(&task).await.unwrap();

        let created = mock.created_issues.lock().unwrap();
        assert_eq!(created.len(), 1);
    }

    #[tokio::test]
    async fn try_push_task_noop_when_no_link() {
        let pool = setup_db().await;
        let project_id = create_project(&pool).await;
        let task = make_task(project_id, TaskStatus::Todo, TaskPriority::High);
        insert_task(&pool, &task).await;

        let mock = Arc::new(MockGitHubApi::new());
        let engine = SyncEngine::new(Arc::clone(&mock) as Arc<dyn GitHubApi>, pool.clone());

        // Should succeed (no-op) without a link
        engine.try_push_task(&task).await.unwrap();

        let created = mock.created_issues.lock().unwrap();
        assert!(created.is_empty());
    }

    // --- detect_pull_requests tests ---

    fn make_linked_pr(pr_number: u64, state: &str, is_merged: bool) -> LinkedPr {
        LinkedPr {
            pr_number,
            title: format!("PR #{pr_number}"),
            url: format!("https://github.com/owner/repo/pull/{pr_number}"),
            state: state.to_string(),
            is_merged,
            author: Some("octocat".to_string()),
        }
    }

    #[tokio::test]
    async fn detect_prs_saves_prs_for_mapped_issues() {
        let pool = setup_db().await;
        let project_id = create_project(&pool).await;
        let link = make_link(project_id);
        insert_link(&pool, &link).await;
        let task = make_task(project_id, TaskStatus::Todo, TaskPriority::Medium);
        insert_task(&pool, &task).await;

        // Create mapping: task <-> issue #10
        github_sync_service::create_mapping(&pool, task.id, link.id, 10, Some(10000))
            .await
            .unwrap();

        let mock = MockGitHubApi::new();
        mock.linked_prs
            .lock()
            .unwrap()
            .insert(10, vec![make_linked_pr(99, "open", false)]);
        let engine = SyncEngine::new(Arc::new(mock) as Arc<dyn GitHubApi>, pool.clone());

        engine.detect_pull_requests(&link).await;

        let prs = github_pr_service::list_prs_for_task(&pool, task.id)
            .await
            .unwrap();
        assert_eq!(prs.len(), 1);
        assert_eq!(prs[0].pr_number, 99);
        assert_eq!(prs[0].author.as_deref(), Some("octocat"));
    }

    #[tokio::test]
    async fn detect_prs_skips_unmapped_issues() {
        let pool = setup_db().await;
        let project_id = create_project(&pool).await;
        let link = make_link(project_id);
        insert_link(&pool, &link).await;

        // No mappings exist — detect should do nothing
        let mock = Arc::new(MockGitHubApi::new());
        let engine = SyncEngine::new(mock as Arc<dyn GitHubApi>, pool.clone());

        engine.detect_pull_requests(&link).await;
        // No assertions needed beyond no-panic
    }

    #[tokio::test]
    async fn detect_prs_updates_existing_pr() {
        let pool = setup_db().await;
        let project_id = create_project(&pool).await;
        let link = make_link(project_id);
        insert_link(&pool, &link).await;
        let task = make_task(project_id, TaskStatus::Todo, TaskPriority::Medium);
        insert_task(&pool, &task).await;

        github_sync_service::create_mapping(&pool, task.id, link.id, 10, Some(10000))
            .await
            .unwrap();

        // First detection: open PR
        let mock = MockGitHubApi::new();
        mock.linked_prs
            .lock()
            .unwrap()
            .insert(10, vec![make_linked_pr(99, "open", false)]);
        let engine = SyncEngine::new(Arc::new(mock) as Arc<dyn GitHubApi>, pool.clone());
        engine.detect_pull_requests(&link).await;

        // Second detection: same PR now merged
        let mock2 = MockGitHubApi::new();
        mock2
            .linked_prs
            .lock()
            .unwrap()
            .insert(10, vec![make_linked_pr(99, "closed", true)]);
        let engine2 = SyncEngine::new(Arc::new(mock2) as Arc<dyn GitHubApi>, pool.clone());
        engine2.detect_pull_requests(&link).await;

        let prs = github_pr_service::list_prs_for_task(&pool, task.id)
            .await
            .unwrap();
        assert_eq!(prs.len(), 1);
        assert!(prs[0].is_merged);
    }

    #[tokio::test]
    async fn sync_project_includes_pr_detection() {
        let pool = setup_db().await;
        let project_id = create_project(&pool).await;
        let link = make_link(project_id);
        insert_link(&pool, &link).await;
        let task = make_task(project_id, TaskStatus::Todo, TaskPriority::Medium);
        insert_task(&pool, &task).await;

        // After push, the mock will create issue #1. Set up PR for issue #1.
        let mock = MockGitHubApi::new();
        mock.linked_prs
            .lock()
            .unwrap()
            .insert(1, vec![make_linked_pr(50, "open", false)]);
        let engine = SyncEngine::new(Arc::new(mock) as Arc<dyn GitHubApi>, pool.clone());

        let result = engine.sync_project(&link).await.unwrap();
        assert_eq!(result.pushed, 1);

        // PR should have been detected and saved
        let prs = github_pr_service::list_prs_for_task(&pool, task.id)
            .await
            .unwrap();
        assert_eq!(prs.len(), 1);
        assert_eq!(prs[0].pr_number, 50);
    }
}
