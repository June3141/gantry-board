use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use sqlx::SqlitePool;
use tokio::sync::Mutex;
use tracing::warn;
use uuid::Uuid;

use crate::agent::executor::{AgentConfig, AgentExecutor, AgentOutputEvent};
use crate::error::{AppError, AppResult};
use crate::models::agent_session::{
    AgentSessionStatus, AgentType, CreateAgentSessionRequest, UpdateAgentSessionRequest,
};
use crate::services::{agent_session_output_service, agent_session_service, worktree_service};
use crate::sse::event::SseEvent;
use crate::sse::hub::SseHub;

struct RunningSession {
    cancel: tokio_util::sync::CancellationToken,
    _monitor_handle: tokio::task::JoinHandle<()>,
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
    executors: HashMap<AgentType, Arc<dyn AgentExecutor>>,
    pool: SqlitePool,
    repo_path: PathBuf,
    sse_hub: Arc<SseHub>,
    /// Per-task lock to prevent concurrent start_session for the same task.
    task_locks: Mutex<HashMap<Uuid, Arc<Mutex<()>>>>,
    /// Currently running sessions, keyed by session_id.
    running: Arc<Mutex<HashMap<Uuid, RunningSession>>>,
}

impl AgentOrchestrator {
    pub fn new(
        executors: HashMap<AgentType, Arc<dyn AgentExecutor>>,
        pool: SqlitePool,
        repo_path: PathBuf,
        sse_hub: Arc<SseHub>,
    ) -> Self {
        Self {
            executors,
            pool,
            repo_path,
            sse_hub,
            task_locks: Mutex::new(HashMap::new()),
            running: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Start a new agent session for a task.
    ///
    /// 1. Acquire per-task lock (atomic duplicate prevention)
    /// 2. Validate executor exists (fail fast before allocating resources)
    /// 3. Check no active session for this task
    /// 4. Create DB session (Pending)
    /// 5. Create git worktree (via spawn_blocking)
    /// 6. Start agent executor
    /// 7. Update DB session (Running)
    /// 8. Spawn background monitor for output_rx
    /// 9. Register in running sessions map
    pub async fn start_session(&self, req: StartSessionRequest) -> AppResult<StartSessionResult> {
        // Step 1: Acquire per-task lock
        let task_lock = {
            let mut locks = self.task_locks.lock().await;
            locks
                .entry(req.task_id)
                .or_insert_with(|| Arc::new(Mutex::new(())))
                .clone()
        };
        let _guard = task_lock.lock().await;

        // Step 2: Validate executor exists before allocating resources
        let executor = self
            .executors
            .get(&req.agent_type)
            .ok_or_else(|| {
                AppError::Validation(format!("unsupported agent type: {}", req.agent_type))
            })?
            .clone();

        // Steps 3-4b: Wrapped in transaction for atomicity
        let session = {
            let mut tx = self
                .pool
                .begin()
                .await
                .map_err(|e| AppError::Internal(format!("failed to begin transaction: {e}")))?;

            // Step 3: Duplicate prevention
            agent_session_service::check_no_active_session_tx(&mut tx, req.task_id).await?;

            // Step 4: Create DB session (Pending)
            let session = agent_session_service::create_agent_session_tx(
                &mut tx,
                req.task_id,
                &CreateAgentSessionRequest {
                    agent_type: req.agent_type.clone(),
                },
            )
            .await?;

            // Step 4b: Save prompt for restart support
            agent_session_service::save_prompt_tx(&mut tx, session.id, &req.prompt).await?;

            tx.commit()
                .await
                .map_err(|e| AppError::Internal(format!("failed to commit transaction: {e}")))?;
            session
        };

        // Step 5: Create worktree (spawn_blocking for synchronous git2 operations)
        let worktree_name = format!("task-{}-session-{}", req.task_id, session.id);
        let repo_path = self.repo_path.clone();
        let wt_name = worktree_name.clone();
        let worktree = match tokio::task::spawn_blocking(move || {
            worktree_service::create_worktree(&repo_path, &wt_name)
        })
        .await
        .map_err(|e| AppError::Internal(format!("worktree task panicked: {e}")))?
        {
            Ok(wt) => wt,
            Err(e) => {
                let _ = self.mark_session_cancelled(req.task_id, session.id).await;
                return Err(e);
            }
        };

        // Step 6: Start executor
        let config = AgentConfig {
            agent_type: req.agent_type,
            session_id: session.id,
            task_id: req.task_id,
            working_dir: worktree.path.clone(),
            prompt: req.prompt,
            allowed_tools: vec![],
        };

        let handle = match executor.start(config).await {
            Ok(h) => h,
            Err(e) => {
                let _ = Self::delete_worktree_blocking(&self.repo_path, &worktree_name).await;
                let _ = self.mark_session_cancelled(req.task_id, session.id).await;
                return Err(e);
            }
        };

        // Step 7: Update DB session to Running
        let running_session = match agent_session_service::update_agent_session(
            &self.pool,
            req.task_id,
            session.id,
            &UpdateAgentSessionRequest {
                status: AgentSessionStatus::Running,
            },
        )
        .await
        {
            Ok(s) => s,
            Err(e) => {
                // Rollback: cancel executor + delete worktree + mark cancelled
                handle.cancel.cancel();
                let _ = Self::delete_worktree_blocking(&self.repo_path, &worktree_name).await;
                let _ = self.mark_session_cancelled(req.task_id, session.id).await;
                return Err(e);
            }
        };

        // Broadcast status change to Running
        self.sse_hub
            .broadcast(SseEvent::agent_session_status_changed(running_session));

        // Step 8: Spawn background monitor to drain output_rx and update DB on completion
        let monitor_handle = self.spawn_session_monitor(
            handle.output_rx,
            handle.join_handle,
            handle.cancel.clone(),
            req.task_id,
            session.id,
            self.repo_path.clone(),
            worktree_name,
        );

        // Step 9: Register in running sessions map
        let worktree_path = worktree.path.clone();
        {
            let mut running = self.running.lock().await;
            running.insert(
                session.id,
                RunningSession {
                    cancel: handle.cancel,
                    _monitor_handle: monitor_handle,
                },
            );
        }

        Ok(StartSessionResult {
            session_id: session.id,
            worktree_path,
        })
    }

    /// Stop a running agent session.
    ///
    /// 1. Cancel the agent process
    /// 2. Update DB session (Cancelled)
    /// 3. Remove from running map only after DB update succeeds
    pub async fn stop_session(&self, task_id: Uuid, session_id: Uuid) -> AppResult<()> {
        // Step 1: Check session exists and cancel it (keep in map)
        {
            let running = self.running.lock().await;
            let session = running.get(&session_id).ok_or_else(|| {
                AppError::NotFound(format!("no running session found: {session_id}"))
            })?;
            session.cancel.cancel();
        }

        // Step 2: Update DB session to Cancelled
        agent_session_service::update_agent_session(
            &self.pool,
            task_id,
            session_id,
            &UpdateAgentSessionRequest {
                status: AgentSessionStatus::Cancelled,
            },
        )
        .await?;

        // Step 3: Remove from running map after DB success
        {
            let mut running = self.running.lock().await;
            running.remove(&session_id);
        }

        Ok(())
    }

    pub async fn is_running(&self, session_id: Uuid) -> bool {
        let running = self.running.lock().await;
        running.contains_key(&session_id)
    }

    async fn mark_session_cancelled(&self, task_id: Uuid, session_id: Uuid) -> AppResult<()> {
        // Pending -> Failed is not a valid transition; use Pending -> Cancelled.
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

    async fn delete_worktree_blocking(repo_path: &Path, name: &str) -> AppResult<()> {
        let repo = repo_path.to_path_buf();
        let n = name.to_string();
        tokio::task::spawn_blocking(move || worktree_service::delete_worktree(&repo, &n))
            .await
            .map_err(|e| AppError::Internal(format!("worktree delete task panicked: {e}")))?
    }

    /// Spawn a background task that drains `output_rx` and updates DB status
    /// when the agent completes or fails naturally (not via stop_session).
    #[allow(clippy::too_many_arguments)]
    fn spawn_session_monitor(
        &self,
        mut output_rx: tokio::sync::mpsc::Receiver<AgentOutputEvent>,
        join_handle: tokio::task::JoinHandle<AppResult<()>>,
        cancel: tokio_util::sync::CancellationToken,
        task_id: Uuid,
        session_id: Uuid,
        repo_path: PathBuf,
        worktree_name: String,
    ) -> tokio::task::JoinHandle<()> {
        let pool = self.pool.clone();
        let running = Arc::clone(&self.running);
        let sse_hub = Arc::clone(&self.sse_hub);
        tokio::spawn(async move {
            // Track terminal event to determine final status
            let mut final_status = AgentSessionStatus::Completed;
            let mut sequence: i64 = 0;

            // Drain output events until the channel closes
            while let Some(event) = output_rx.recv().await {
                match event {
                    AgentOutputEvent::Completed => break,
                    AgentOutputEvent::Failed { .. } => {
                        final_status = AgentSessionStatus::Failed;
                        break;
                    }
                    AgentOutputEvent::Output { text } => {
                        sse_hub.broadcast(SseEvent::agent_output(session_id, text.clone()));
                        // Best-effort persistence (after broadcast to avoid delaying SSE)
                        if let Err(e) = agent_session_output_service::append_output(
                            &pool, session_id, sequence, &text,
                        )
                        .await
                        {
                            warn!("failed to persist output for session {session_id} seq {sequence}: {e}");
                        }
                        sequence += 1;
                    }
                }
            }

            // Wait for the executor task to finish
            let _ = join_handle.await;

            // If cancelled by stop_session, don't update DB (stop_session handles it)
            if cancel.is_cancelled() {
                return;
            }

            // Natural completion: update DB and broadcast (best-effort)
            match agent_session_service::update_agent_session(
                &pool,
                task_id,
                session_id,
                &UpdateAgentSessionRequest {
                    status: final_status.clone(),
                },
            )
            .await
            {
                Ok(session) => {
                    sse_hub.broadcast(SseEvent::agent_session_status_changed(session));
                }
                Err(e) => {
                    warn!("failed to update session {session_id} status: {e}");
                }
            }

            // Cleanup worktree after session completion
            let rp = repo_path;
            let wn = worktree_name;
            if let Err(e) = Self::delete_worktree_blocking(&rp, &wn).await {
                warn!("failed to cleanup worktree {wn} after session completion: {e}");
            }

            // Remove from running map
            let mut map = running.lock().await;
            map.remove(&session_id);
        })
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
        create_orchestrator_with_hub(executor, pool, repo_path, sse_hub)
    }

    fn create_orchestrator_with_hub(
        executor: Arc<dyn AgentExecutor>,
        pool: SqlitePool,
        repo_path: PathBuf,
        sse_hub: Arc<SseHub>,
    ) -> AgentOrchestrator {
        let mut executors = HashMap::new();
        executors.insert(AgentType::ClaudeCode, executor);
        AgentOrchestrator::new(executors, pool, repo_path, sse_hub)
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

        // Verify worktree was cleaned up (name includes session_id)
        let session_id = sessions[0].id;
        let worktree_name = format!("task-{task_id}-session-{session_id}");
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

    #[tokio::test]
    async fn test_start_session_broadcasts_status_change() {
        let pool = setup_test_db().await;
        let (_project_id, task_id) = create_test_task(&pool).await;
        let tmp = tempfile::tempdir().unwrap();
        let repo_path = init_test_repo(tmp.path());

        let sse_hub = Arc::new(SseHub::new(16));
        let mut rx = sse_hub.subscribe();

        let executor = Arc::new(MockExecutor::new());
        let orchestrator =
            create_orchestrator_with_hub(executor, pool.clone(), repo_path, Arc::clone(&sse_hub));

        orchestrator
            .start_session(StartSessionRequest {
                task_id,
                agent_type: AgentType::ClaudeCode,
                prompt: "test".to_string(),
            })
            .await
            .expect("start should succeed");

        // Should receive a status changed event (Running)
        let event = tokio::time::timeout(std::time::Duration::from_secs(1), rx.recv())
            .await
            .expect("should receive event within timeout")
            .expect("recv should succeed");

        assert_eq!(event.event_type(), "agent_session_status_changed");
    }

    #[tokio::test]
    async fn test_agent_output_is_broadcast_to_sse() {
        use crate::agent::executor::AgentHandle;

        /// Mock executor that sends output events before completing.
        struct OutputExecutor;

        #[async_trait::async_trait]
        impl AgentExecutor for OutputExecutor {
            async fn start(&self, _config: AgentConfig) -> AppResult<AgentHandle> {
                let cancel = CancellationToken::new();
                let (tx, rx) = mpsc::channel(16);
                let join_handle = tokio::spawn(async move {
                    let _ = tx
                        .send(AgentOutputEvent::Output {
                            text: "hello world".to_string(),
                        })
                        .await;
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

        let pool = setup_test_db().await;
        let (_project_id, task_id) = create_test_task(&pool).await;
        let tmp = tempfile::tempdir().unwrap();
        let repo_path = init_test_repo(tmp.path());

        let sse_hub = Arc::new(SseHub::new(16));
        let mut rx = sse_hub.subscribe();

        let executor = Arc::new(OutputExecutor);
        let orchestrator =
            create_orchestrator_with_hub(executor, pool.clone(), repo_path, Arc::clone(&sse_hub));

        orchestrator
            .start_session(StartSessionRequest {
                task_id,
                agent_type: AgentType::ClaudeCode,
                prompt: "test".to_string(),
            })
            .await
            .expect("start should succeed");

        // Collect events (status_changed for Running, agent_output, status_changed for Completed)
        let mut event_types = Vec::new();
        for _ in 0..3 {
            match tokio::time::timeout(std::time::Duration::from_secs(2), rx.recv()).await {
                Ok(Ok(event)) => event_types.push(event.event_type().to_string()),
                _ => break,
            }
        }

        assert!(
            event_types.contains(&"agent_output".to_string()),
            "expected agent_output event, got: {event_types:?}"
        );
        assert!(
            event_types.contains(&"agent_session_status_changed".to_string()),
            "expected agent_session_status_changed event, got: {event_types:?}"
        );
    }

    #[tokio::test]
    async fn test_agent_output_is_persisted_to_db() {
        use crate::agent::executor::AgentHandle;
        use crate::services::agent_session_output_service;

        /// Mock executor that sends multiple output events before completing.
        struct PersistingOutputExecutor;

        #[async_trait::async_trait]
        impl AgentExecutor for PersistingOutputExecutor {
            async fn start(&self, _config: AgentConfig) -> AppResult<AgentHandle> {
                let cancel = CancellationToken::new();
                let (tx, rx) = mpsc::channel(16);
                let join_handle = tokio::spawn(async move {
                    let _ = tx
                        .send(AgentOutputEvent::Output {
                            text: "line one".to_string(),
                        })
                        .await;
                    let _ = tx
                        .send(AgentOutputEvent::Output {
                            text: "line two".to_string(),
                        })
                        .await;
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

        let pool = setup_test_db().await;
        let (_project_id, task_id) = create_test_task(&pool).await;
        let tmp = tempfile::tempdir().unwrap();
        let repo_path = init_test_repo(tmp.path());

        let sse_hub = Arc::new(SseHub::new(16));
        let mut rx = sse_hub.subscribe();

        let executor = Arc::new(PersistingOutputExecutor);
        let orchestrator =
            create_orchestrator_with_hub(executor, pool.clone(), repo_path, Arc::clone(&sse_hub));

        let result = orchestrator
            .start_session(StartSessionRequest {
                task_id,
                agent_type: AgentType::ClaudeCode,
                prompt: "test".to_string(),
            })
            .await
            .expect("start should succeed");

        // Wait for all events: Running status + 2 outputs + Completed status = 4
        for _ in 0..4 {
            tokio::time::timeout(std::time::Duration::from_secs(2), rx.recv())
                .await
                .expect("timed out waiting for SSE event")
                .expect("SSE sender dropped");
        }

        // Poll DB until outputs are persisted (deterministic wait)
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
        let outputs = loop {
            let outputs = agent_session_output_service::get_outputs(&pool, result.session_id)
                .await
                .expect("should get outputs");
            if outputs.len() == 2 {
                break outputs;
            }
            assert!(
                std::time::Instant::now() < deadline,
                "timed out waiting for outputs to be persisted"
            );
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        };
        assert_eq!(outputs[0].sequence, 0);
        assert_eq!(outputs[0].content, "line one");
        assert_eq!(outputs[1].sequence, 1);
        assert_eq!(outputs[1].content, "line two");
    }

    #[tokio::test]
    async fn test_start_session_unsupported_agent_type() {
        let pool = setup_test_db().await;
        let (_project_id, task_id) = create_test_task(&pool).await;
        let tmp = tempfile::tempdir().unwrap();
        let repo_path = init_test_repo(tmp.path());

        // Register only ClaudeCode executor
        let executor = Arc::new(MockExecutor::new());
        let orchestrator = create_orchestrator(executor, pool.clone(), repo_path);

        // Request GeminiCli which is not registered
        let result = orchestrator
            .start_session(StartSessionRequest {
                task_id,
                agent_type: AgentType::GeminiCli,
                prompt: "test".to_string(),
            })
            .await;

        assert!(
            matches!(result, Err(AppError::Validation(_))),
            "expected Validation error for unsupported agent type, got: {result:?}"
        );

        // Verify no DB session was created (early validation prevents resource leaks)
        let sessions = agent_session_service::list_agent_sessions(&pool, task_id)
            .await
            .unwrap();
        assert!(
            sessions.is_empty(),
            "no DB session should be created for unsupported agent type"
        );
    }
}
