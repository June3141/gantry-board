use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use sqlx::SqlitePool;
use tokio::sync::Mutex;
use uuid::Uuid;

use crate::agent::executor::{AgentConfig, AgentExecutor, AgentHandle};
use crate::error::{AppError, AppResult};
use crate::models::agent_session::{
    AgentSessionStatus, AgentType, CreateAgentSessionRequest, UpdateAgentSessionRequest,
};
use crate::services::{agent_session_service, worktree_service};
use crate::sse::hub::SseHub;

struct RunningSession {
    handle: AgentHandle,
}

/// Parameters for starting a new agent session.
#[derive(Debug)]
pub struct StartSessionRequest {
    pub task_id: Uuid,
    pub agent_type: AgentType,
    pub prompt: String,
}

/// Result of a successful session start.
#[derive(Debug)]
pub struct StartSessionResult {
    pub session_id: Uuid,
    pub worktree_path: PathBuf,
}

/// Orchestrates agent session lifecycle:
/// DB session creation → worktree setup → executor launch → status updates → cleanup.
pub struct AgentOrchestrator {
    executor: Arc<dyn AgentExecutor>,
    pool: SqlitePool,
    repo_path: PathBuf,
    _sse_hub: Arc<SseHub>,
    running: Mutex<HashMap<Uuid, RunningSession>>,
}

impl AgentOrchestrator {
    pub fn new(
        executor: Arc<dyn AgentExecutor>,
        pool: SqlitePool,
        repo_path: PathBuf,
        sse_hub: Arc<SseHub>,
    ) -> Self {
        Self {
            executor,
            pool,
            repo_path,
            _sse_hub: sse_hub,
            running: Mutex::new(HashMap::new()),
        }
    }

    /// Start a new agent session for a task.
    ///
    /// 1. Check no active session for this task (duplicate prevention)
    /// 2. Create DB session (Pending)
    /// 3. Create git worktree
    /// 4. Start agent executor
    /// 5. Update DB session (Running)
    /// 6. Register in running sessions map
    pub async fn start_session(&self, req: StartSessionRequest) -> AppResult<StartSessionResult> {
        // Step 1: Duplicate prevention
        self.check_no_active_session(req.task_id).await?;

        // Step 2: Create DB session
        let session = agent_session_service::create_agent_session(
            &self.pool,
            req.task_id,
            &CreateAgentSessionRequest {
                agent_type: req.agent_type.clone(),
            },
        )
        .await?;

        // Step 3: Create worktree
        let worktree_name = format!("task-{}", req.task_id);
        let worktree = match worktree_service::create_worktree(&self.repo_path, &worktree_name) {
            Ok(wt) => wt,
            Err(e) => {
                let _ = self.mark_session_failed(req.task_id, session.id).await;
                return Err(e);
            }
        };

        // Step 4: Start executor
        let config = AgentConfig {
            agent_type: req.agent_type,
            session_id: session.id,
            task_id: req.task_id,
            working_dir: worktree.path.clone(),
            prompt: req.prompt,
        };

        let handle = match self.executor.start(config).await {
            Ok(h) => h,
            Err(e) => {
                let _ = worktree_service::delete_worktree(&self.repo_path, &worktree_name);
                let _ = self.mark_session_failed(req.task_id, session.id).await;
                return Err(e);
            }
        };

        // Step 5: Update DB session to Running
        agent_session_service::update_agent_session(
            &self.pool,
            req.task_id,
            session.id,
            &UpdateAgentSessionRequest {
                status: AgentSessionStatus::Running,
            },
        )
        .await?;

        // Step 6: Register in running sessions map
        let worktree_path = worktree.path.clone();
        {
            let mut running = self.running.lock().await;
            running.insert(session.id, RunningSession { handle });
        }

        Ok(StartSessionResult {
            session_id: session.id,
            worktree_path,
        })
    }

    /// Stop a running agent session.
    ///
    /// 1. Remove from running map
    /// 2. Cancel the agent process
    /// 3. Update DB session (Cancelled)
    pub async fn stop_session(&self, task_id: Uuid, session_id: Uuid) -> AppResult<()> {
        let running_session = {
            let mut running = self.running.lock().await;
            running.remove(&session_id)
        };

        let running_session = running_session
            .ok_or_else(|| AppError::NotFound(format!("no running session found: {session_id}")))?;

        running_session.handle.cancel.cancel();

        agent_session_service::update_agent_session(
            &self.pool,
            task_id,
            session_id,
            &UpdateAgentSessionRequest {
                status: AgentSessionStatus::Cancelled,
            },
        )
        .await?;

        Ok(())
    }

    pub async fn is_running(&self, session_id: Uuid) -> bool {
        let running = self.running.lock().await;
        running.contains_key(&session_id)
    }

    async fn check_no_active_session(&self, task_id: Uuid) -> AppResult<()> {
        let sessions = agent_session_service::list_agent_sessions(&self.pool, task_id).await?;
        let has_active = sessions.iter().any(|s| {
            matches!(
                s.status,
                AgentSessionStatus::Pending | AgentSessionStatus::Running
            )
        });
        if has_active {
            return Err(AppError::Conflict(format!(
                "task {task_id} already has an active agent session"
            )));
        }
        Ok(())
    }

    async fn mark_session_failed(&self, task_id: Uuid, session_id: Uuid) -> AppResult<()> {
        // Pending -> Failed is not a valid transition in agent_session_service.
        // We need Pending -> Cancelled instead.
        agent_session_service::update_agent_session(
            &self.pool,
            task_id,
            session_id,
            &UpdateAgentSessionRequest {
                status: AgentSessionStatus::Cancelled,
            },
        )
        .await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::executor::{AgentHandle, AgentOutputEvent};
    use crate::error::AppError;
    use crate::models::agent_session::AgentType;
    use crate::models::project::CreateProjectRequest;
    use crate::models::task::CreateTaskRequest;
    use crate::services::{project_service, task_service};
    use crate::test_helpers::setup_test_db;
    use std::path::Path;
    use std::sync::atomic::{AtomicBool, Ordering};
    use tokio::sync::mpsc;
    use tokio_util::sync::CancellationToken;

    /// Mock executor that always succeeds.
    struct MockExecutor {
        started: Arc<AtomicBool>,
    }

    impl MockExecutor {
        fn new() -> Self {
            Self {
                started: Arc::new(AtomicBool::new(false)),
            }
        }
    }

    #[async_trait::async_trait]
    impl AgentExecutor for MockExecutor {
        async fn start(&self, _config: AgentConfig) -> AppResult<AgentHandle> {
            self.started.store(true, Ordering::SeqCst);
            let cancel = CancellationToken::new();
            let (tx, rx) = mpsc::channel(16);
            let token = cancel.clone();
            let join_handle = tokio::spawn(async move {
                token.cancelled().await;
                let _ = tx.send(AgentOutputEvent::Completed).await;
                Ok(())
            });
            Ok(AgentHandle {
                cancel,
                output_rx: rx,
                join_handle,
            })
        }
    }

    /// Mock executor that always fails on start.
    struct FailingExecutor;

    #[async_trait::async_trait]
    impl AgentExecutor for FailingExecutor {
        async fn start(&self, _config: AgentConfig) -> AppResult<AgentHandle> {
            Err(AppError::Internal("executor failed to start".into()))
        }
    }

    /// Create a test task in DB and return (project_id, task_id).
    async fn create_test_task(pool: &SqlitePool) -> (Uuid, Uuid) {
        let project = project_service::create_project(
            pool,
            &CreateProjectRequest {
                name: "Test Project".to_string(),
                description: None,
            },
        )
        .await
        .expect("Failed to create project");

        let task = task_service::create_task(
            pool,
            &CreateTaskRequest {
                project_id: project.id,
                title: "Test Task".to_string(),
                description: None,
                status: None,
                priority: None,
                parent_id: None,
                assigned_to: None,
            },
        )
        .await
        .expect("Failed to create task");

        (project.id, task.id)
    }

    /// Initialize a bare git repo and return its path.
    fn init_test_repo(dir: &Path) -> PathBuf {
        let repo_path = dir.join("repo");
        let repo = git2::Repository::init(&repo_path).expect("Failed to init repo");
        // Create an initial commit so HEAD exists
        let sig = git2::Signature::now("test", "test@test.com").unwrap();
        let tree_id = repo.index().unwrap().write_tree().unwrap();
        let tree = repo.find_tree(tree_id).unwrap();
        repo.commit(Some("HEAD"), &sig, &sig, "initial", &tree, &[])
            .unwrap();
        repo_path
    }

    fn create_orchestrator(
        executor: Arc<dyn AgentExecutor>,
        pool: SqlitePool,
        repo_path: PathBuf,
    ) -> AgentOrchestrator {
        let sse_hub = Arc::new(SseHub::new(16));
        AgentOrchestrator::new(executor, pool, repo_path, sse_hub)
    }

    #[tokio::test]
    async fn test_start_session_success() {
        let pool = setup_test_db().await;
        let (_project_id, task_id) = create_test_task(&pool).await;
        let tmp = tempfile::tempdir().unwrap();
        let repo_path = init_test_repo(tmp.path());

        let executor = Arc::new(MockExecutor::new());
        let orchestrator = create_orchestrator(executor.clone(), pool.clone(), repo_path.clone());

        let result = orchestrator
            .start_session(StartSessionRequest {
                task_id,
                agent_type: AgentType::ClaudeCode,
                prompt: "test prompt".to_string(),
            })
            .await;

        let result = result.expect("start_session should succeed");
        assert!(result.worktree_path.exists());
        assert!(executor.started.load(Ordering::SeqCst));

        // Verify DB session is Running
        let sessions = agent_session_service::list_agent_sessions(&pool, task_id)
            .await
            .unwrap();
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].status, AgentSessionStatus::Running);
        assert!(sessions[0].started_at.is_some());
    }

    #[tokio::test]
    async fn test_stop_session_success() {
        let pool = setup_test_db().await;
        let (_project_id, task_id) = create_test_task(&pool).await;
        let tmp = tempfile::tempdir().unwrap();
        let repo_path = init_test_repo(tmp.path());

        let executor = Arc::new(MockExecutor::new());
        let orchestrator = create_orchestrator(executor, pool.clone(), repo_path);

        let start_result = orchestrator
            .start_session(StartSessionRequest {
                task_id,
                agent_type: AgentType::ClaudeCode,
                prompt: "test".to_string(),
            })
            .await
            .expect("start should succeed");

        let stop_result = orchestrator
            .stop_session(task_id, start_result.session_id)
            .await;

        stop_result.expect("stop_session should succeed");

        // Verify DB session is Cancelled
        let session =
            agent_session_service::get_agent_session(&pool, task_id, start_result.session_id)
                .await
                .unwrap();
        assert_eq!(session.status, AgentSessionStatus::Cancelled);
        assert!(session.finished_at.is_some());
    }

    #[tokio::test]
    async fn test_start_session_rejects_duplicate() {
        let pool = setup_test_db().await;
        let (_project_id, task_id) = create_test_task(&pool).await;
        let tmp = tempfile::tempdir().unwrap();
        let repo_path = init_test_repo(tmp.path());

        let executor = Arc::new(MockExecutor::new());
        let orchestrator = create_orchestrator(executor, pool.clone(), repo_path);

        // First start should succeed
        orchestrator
            .start_session(StartSessionRequest {
                task_id,
                agent_type: AgentType::ClaudeCode,
                prompt: "first".to_string(),
            })
            .await
            .expect("first start should succeed");

        // Second start for same task should fail with Conflict
        let result = orchestrator
            .start_session(StartSessionRequest {
                task_id,
                agent_type: AgentType::ClaudeCode,
                prompt: "second".to_string(),
            })
            .await;

        assert!(matches!(result, Err(AppError::Conflict(_))));
    }

    #[tokio::test]
    async fn test_start_session_executor_failure_cleans_up() {
        let pool = setup_test_db().await;
        let (_project_id, task_id) = create_test_task(&pool).await;
        let tmp = tempfile::tempdir().unwrap();
        let repo_path = init_test_repo(tmp.path());

        let executor = Arc::new(FailingExecutor);
        let orchestrator = create_orchestrator(executor, pool.clone(), repo_path.clone());

        let result = orchestrator
            .start_session(StartSessionRequest {
                task_id,
                agent_type: AgentType::ClaudeCode,
                prompt: "test".to_string(),
            })
            .await;

        assert!(result.is_err());

        // Verify DB session is Cancelled (Pending → Failed is not a valid transition)
        let sessions = agent_session_service::list_agent_sessions(&pool, task_id)
            .await
            .unwrap();
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].status, AgentSessionStatus::Cancelled);

        // Verify worktree was cleaned up
        let worktree_name = format!("task-{task_id}");
        let worktree_result = worktree_service::get_worktree(&repo_path, &worktree_name);
        assert!(worktree_result.is_err());
    }

    #[tokio::test]
    async fn test_stop_session_not_found() {
        let pool = setup_test_db().await;
        let (_project_id, task_id) = create_test_task(&pool).await;
        let tmp = tempfile::tempdir().unwrap();
        let repo_path = init_test_repo(tmp.path());

        let executor = Arc::new(MockExecutor::new());
        let orchestrator = create_orchestrator(executor, pool, repo_path);

        let result = orchestrator.stop_session(task_id, Uuid::new_v4()).await;

        assert!(matches!(result, Err(AppError::NotFound(_))));
    }

    #[tokio::test]
    async fn test_is_running_state() {
        let pool = setup_test_db().await;
        let (_project_id, task_id) = create_test_task(&pool).await;
        let tmp = tempfile::tempdir().unwrap();
        let repo_path = init_test_repo(tmp.path());

        let executor = Arc::new(MockExecutor::new());
        let orchestrator = create_orchestrator(executor, pool, repo_path);

        let random_id = Uuid::new_v4();
        assert!(!orchestrator.is_running(random_id).await);

        let result = orchestrator
            .start_session(StartSessionRequest {
                task_id,
                agent_type: AgentType::ClaudeCode,
                prompt: "test".to_string(),
            })
            .await
            .expect("start should succeed");

        assert!(orchestrator.is_running(result.session_id).await);

        orchestrator
            .stop_session(task_id, result.session_id)
            .await
            .expect("stop should succeed");

        assert!(!orchestrator.is_running(result.session_id).await);
    }
}
