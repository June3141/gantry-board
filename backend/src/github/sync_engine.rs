use chrono::Utc;
use sqlx::SqlitePool;
use std::sync::Arc;

use super::api::GitHubApi;
use super::label_mapping;
use crate::error::AppResult;
use crate::models::github::{
    CreateIssueRequest, GitHubIssue, GitHubIssueMapping, GitHubLink, SyncResult, UpdateIssueRequest,
};
use crate::models::task::{CreateTaskRequest, Task, TaskPriority, TaskStatus, UpdateTaskRequest};
use crate::services::{github_link_service, github_sync_service, task_service};

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
                github_sync_service::update_mapping_timestamps(
                    &self.pool,
                    mapping.id,
                    Some(Utc::now()),
                    None,
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

                Ok(mapping)
            }
        }
    }

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

    /// Run a full sync for one project: ensure labels, push, pull, update last_synced.
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

        // Update last_synced_at
        github_link_service::update_last_synced(&self.pool, link.project_id).await?;

        Ok(SyncResult {
            project_id: link.project_id,
            pushed,
            pulled: pulled_created + pulled_updated,
        })
    }

    /// Sync all projects that have sync enabled.
    pub async fn sync_all(&self) -> AppResult<Vec<SyncResult>> {
        let links = github_link_service::list_sync_enabled(&self.pool).await?;
        let mut results = Vec::new();
        for link in &links {
            let result = self.sync_project(link).await?;
            results.push(result);
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

    /// Try to push a task to GitHub. No-op if project has no GitHub link.
    pub async fn try_push_task(&self, _task: &Task) -> AppResult<()> {
        todo!()
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::github::{CreateIssueRequest, GitHubIssue, UpdateIssueRequest};
    use crate::models::task::{Task, TaskPriority, TaskStatus};
    use chrono::{DateTime, Utc};
    use sqlx::sqlite::SqlitePoolOptions;
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
    }

    impl MockGitHubApi {
        fn new() -> Self {
            Self {
                created_issues: Mutex::new(vec![]),
                updated_issues: Mutex::new(vec![]),
                ensured_labels: Mutex::new(vec![]),
                issues: Mutex::new(vec![]),
                next_number: Mutex::new(1),
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
}
