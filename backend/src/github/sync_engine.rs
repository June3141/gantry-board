use sqlx::SqlitePool;
use std::sync::Arc;

use super::api::GitHubApi;
use crate::error::AppResult;
use crate::models::github::{GitHubIssueMapping, GitHubLink};
use crate::models::task::Task;

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
        _task: &Task,
        _link: &GitHubLink,
    ) -> AppResult<GitHubIssueMapping> {
        todo!()
    }

    /// Ensure all gantry labels exist in the repository.
    pub async fn ensure_all_labels(&self, _owner: &str, _repo: &str) -> AppResult<()> {
        todo!()
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
        /// Next issue number to assign.
        next_number: Mutex<u64>,
    }

    impl MockGitHubApi {
        fn new() -> Self {
            Self {
                created_issues: Mutex::new(vec![]),
                updated_issues: Mutex::new(vec![]),
                ensured_labels: Mutex::new(vec![]),
                next_number: Mutex::new(1),
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
            Ok(vec![])
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

    async fn insert_task(pool: &SqlitePool, task: &Task) {
        sqlx::query(
            "INSERT INTO tasks (id, project_id, title, description, status, priority, position) VALUES ($1, $2, $3, $4, $5, $6, $7)",
        )
        .bind(task.id.to_string())
        .bind(task.project_id.to_string())
        .bind(&task.title)
        .bind(&task.description)
        .bind(format!("{:?}", task.status).to_lowercase())
        .bind(format!("{:?}", task.priority).to_lowercase())
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
}
