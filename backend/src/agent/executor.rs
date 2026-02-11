use std::path::PathBuf;

use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use crate::error::{AppError, AppResult};
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

/// Validate agent configuration before execution.
///
/// Checks that:
/// - `allowed_tools` contain only safe characters (alphanumeric, underscore, hyphen)
/// - `working_dir` exists and is a directory
pub fn validate_config(config: &AgentConfig) -> AppResult<()> {
    // Validate allowed_tools: only alphanumeric, underscore, hyphen allowed
    for tool in &config.allowed_tools {
        if tool.is_empty()
            || !tool
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
        {
            return Err(AppError::Validation(format!(
                "invalid tool name: {tool:?} — only alphanumeric, underscore, and hyphen allowed"
            )));
        }
    }

    // Validate working_dir: must exist and be a directory
    if !config.working_dir.exists() {
        return Err(AppError::Validation(format!(
            "working directory does not exist: {}",
            config.working_dir.display()
        )));
    }
    if !config.working_dir.is_dir() {
        return Err(AppError::Validation(format!(
            "working directory is not a directory: {}",
            config.working_dir.display()
        )));
    }

    Ok(())
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

#[cfg(test)]
mod tests {
    use super::*;

    fn make_config(working_dir: PathBuf, allowed_tools: Vec<String>) -> AgentConfig {
        AgentConfig {
            agent_type: AgentType::ClaudeCode,
            session_id: Uuid::new_v4(),
            task_id: Uuid::new_v4(),
            working_dir,
            prompt: "test".to_string(),
            allowed_tools,
        }
    }

    #[test]
    fn test_validate_accepts_valid_tool_names() {
        let config = make_config(
            std::env::temp_dir(),
            vec!["Read".to_string(), "Write".to_string(), "Bash".to_string()],
        );
        assert!(validate_config(&config).is_ok());
    }

    #[test]
    fn test_validate_accepts_tool_names_with_underscore_and_hyphen() {
        let config = make_config(
            std::env::temp_dir(),
            vec!["my_tool".to_string(), "my-tool".to_string()],
        );
        assert!(validate_config(&config).is_ok());
    }

    #[test]
    fn test_validate_accepts_empty_tools_list() {
        let config = make_config(std::env::temp_dir(), vec![]);
        assert!(validate_config(&config).is_ok());
    }

    #[test]
    fn test_validate_rejects_tool_with_shell_metacharacters() {
        let cases = vec![
            "bash; rm -rf /",
            "tool && echo",
            "tool | cat",
            "$(whoami)",
            "tool`id`",
            "tool name with spaces",
        ];
        for tool_name in cases {
            let config = make_config(std::env::temp_dir(), vec![tool_name.to_string()]);
            let result = validate_config(&config);
            assert!(result.is_err(), "should reject tool name: {tool_name:?}");
        }
    }

    #[test]
    fn test_validate_rejects_empty_tool_name() {
        let config = make_config(std::env::temp_dir(), vec!["".to_string()]);
        assert!(validate_config(&config).is_err());
    }

    #[test]
    fn test_validate_rejects_nonexistent_working_dir() {
        let nonexistent_dir = std::env::temp_dir().join(Uuid::new_v4().to_string());
        let config = make_config(nonexistent_dir, vec![]);
        assert!(validate_config(&config).is_err());
    }

    #[test]
    fn test_validate_accepts_existing_directory() {
        let config = make_config(std::env::temp_dir(), vec![]);
        assert!(validate_config(&config).is_ok());
    }
}
