use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

use tracing::warn;
use uuid::Uuid;

use super::AgentOrchestrator;
use crate::agent::executor::AgentOutputEvent;
use crate::models::agent_session::{AgentSessionStatus, AgentType, UpdateAgentSessionRequest};
use crate::services::agent_session_service;
use crate::sse::event::SseEvent;

impl AgentOrchestrator {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn spawn_session_monitor(
        &self,
        mut output_rx: tokio::sync::mpsc::Receiver<AgentOutputEvent>,
        join_handle: tokio::task::JoinHandle<crate::error::AppResult<()>>,
        cancel: tokio_util::sync::CancellationToken,
        task_id: Uuid,
        session_id: Uuid,
        repo_path: PathBuf,
        worktree_name: String,
        agent_type: AgentType,
    ) -> tokio::task::JoinHandle<()> {
        let pool = self.pool.clone();
        let running = Arc::clone(&self.running);
        let sse_hub = Arc::clone(&self.sse_hub);
        let output_buffer = Arc::clone(&self.output_buffer);
        tokio::spawn(async move {
            let session_start = Instant::now();
            // Track terminal event to determine final status
            let mut final_status = AgentSessionStatus::Completed;
            let mut sequence: i64 = 0;
            let mut persist = true;

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
                        // Best-effort buffered persistence (after broadcast to avoid delaying SSE)
                        if persist {
                            match output_buffer.add(session_id, sequence, text).await {
                                Ok(()) => {}
                                Err(crate::error::AppError::Validation(ref msg))
                                    if msg.contains("limit reached") =>
                                {
                                    warn!("output limit reached for session {session_id}, stopping persistence");
                                    persist = false;
                                }
                                Err(e) => {
                                    warn!("failed to buffer output for session {session_id} seq {sequence}: {e}");
                                }
                            }
                        }
                        sequence += 1;
                    }
                }
            }

            // Flush any remaining buffered outputs before status update
            if let Err(e) = output_buffer.flush().await {
                warn!("failed to flush output buffer for session {session_id}: {e}");
            }

            // Wait for the executor task to finish
            let _ = join_handle.await;

            // If cancelled by stop_session, skip DB update (stop_session handles it)
            // but still perform cleanup below to prevent worktree leaks.
            if !cancel.is_cancelled() {
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
            }

            // Record session duration histogram
            let duration_secs = session_start.elapsed().as_secs_f64();
            metrics::histogram!(crate::observability::metric::AGENT_SESSION_DURATION)
                .record(duration_secs);

            // Record session completion counter
            let status_label = if cancel.is_cancelled() {
                "cancelled"
            } else {
                match &final_status {
                    AgentSessionStatus::Completed => "completed",
                    AgentSessionStatus::Failed => "failed",
                    _ => "unknown",
                }
            };
            metrics::counter!(
                crate::observability::metric::AGENT_SESSIONS_TOTAL,
                "agent_type" => agent_type.to_string(),
                "status" => status_label,
            )
            .increment(1);

            // Always cleanup worktree (both completion and cancellation)
            let rp = repo_path;
            let wn = worktree_name;
            if let Err(e) = Self::delete_worktree_blocking(&rp, &wn).await {
                warn!("failed to cleanup worktree {wn} after session end: {e}");
            }

            // Remove from running map
            let mut map = running.lock().await;
            map.remove(&session_id);
        })
    }
}
