use std::path::PathBuf;

use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use crate::error::AppResult;
use crate::models::agent_session::AgentType;

/// Configuration for launching an agent process.
#[derive(Debug, Clone)]
pub struct AgentConfig {
    pub agent_type: AgentType,
    pub session_id: Uuid,
    pub task_id: Uuid,
    pub working_dir: PathBuf,
    pub prompt: String,
    /// Tool names to pass via `--allowedTools` for Claude Code CLI.
    /// Empty means no restriction (uses CLI's default permission mode).
    pub allowed_tools: Vec<String>,
}

/// Events emitted by a running agent process.
#[derive(Debug, Clone)]
pub enum AgentOutputEvent {
    Output { text: String },
    Completed,
    Failed { error: String },
}

/// Handle to a running agent process.
pub struct AgentHandle {
    pub cancel: CancellationToken,
    pub output_rx: mpsc::Receiver<AgentOutputEvent>,
    pub join_handle: tokio::task::JoinHandle<AppResult<()>>,
}

/// Trait for agent execution backends.
#[async_trait::async_trait]
pub trait AgentExecutor: Send + Sync {
    /// Start an agent process with the given configuration.
    async fn start(&self, config: AgentConfig) -> AppResult<AgentHandle>;
}

/// No-op executor placeholder. Waits for cancellation, then emits Completed.
pub struct NoopExecutor;

#[async_trait::async_trait]
impl AgentExecutor for NoopExecutor {
    async fn start(&self, _config: AgentConfig) -> AppResult<AgentHandle> {
        let cancel = CancellationToken::new();
        let (tx, rx) = mpsc::channel(1);
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
