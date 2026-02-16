use std::path::Path;

use uuid::Uuid;

use super::{AgentOrchestrator, RunningSession};
use crate::error::{AppError, AppResult};
use crate::models::agent_session::{AgentSessionStatus, UpdateAgentSessionRequest};
use crate::services::{agent_session_service, worktree_service};

impl AgentOrchestrator {
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
            metrics::gauge!("gantry_agent_sessions_active").set(running.len() as f64);
        }

        Ok(())
    }

    pub async fn is_running(&self, session_id: Uuid) -> bool {
        let running = self.running.lock().await;
        running.contains_key(&session_id)
    }

    /// Gracefully shut down all running agent sessions.
    ///
    /// Cancels every running session and waits for their monitor tasks to finish
    /// (which handles DB status updates and worktree cleanup).
    pub async fn shutdown_gracefully(&self) {
        let sessions: Vec<(Uuid, RunningSession)> = {
            let mut running = self.running.lock().await;
            let drained = running.drain().collect();
            metrics::gauge!("gantry_agent_sessions_active").set(0.0);
            drained
        };

        if sessions.is_empty() {
            return;
        }

        tracing::info!(
            count = sessions.len(),
            "shutting down running agent sessions"
        );

        let mut handles = Vec::new();
        for (session_id, session) in sessions {
            tracing::info!(%session_id, "cancelling agent session");
            session.cancel.cancel();
            handles.push(session._monitor_handle);
        }

        for handle in handles {
            let _ = handle.await;
        }

        tracing::info!("all agent sessions shut down");
    }

    pub(super) async fn mark_session_cancelled(
        &self,
        task_id: Uuid,
        session_id: Uuid,
    ) -> AppResult<()> {
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

    pub(super) async fn delete_worktree_blocking(repo_path: &Path, name: &str) -> AppResult<()> {
        let repo = repo_path.to_path_buf();
        let n = name.to_string();
        tokio::task::spawn_blocking(move || worktree_service::delete_worktree(&repo, &n))
            .await
            .map_err(|e| AppError::Internal(format!("worktree delete task panicked: {e}")))?
    }
}
