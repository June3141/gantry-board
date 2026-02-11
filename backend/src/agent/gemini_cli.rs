use std::process::Stdio;

use serde::Deserialize;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;
use tracing::warn;

use crate::agent::executor::{
    validate_config, AgentConfig, AgentExecutor, AgentHandle, AgentOutputEvent,
};
use crate::error::{AppError, AppResult};

/// Gemini CLI stream-json event types.
#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
#[serde(rename_all = "snake_case")]
enum GeminiStreamEvent {
    Init {},
    Message {
        role: String,
        content: String,
        #[serde(default)]
        delta: Option<bool>,
    },
    ToolUse {},
    ToolResult {},
    Error {
        message: String,
    },
    Result {
        status: String,
        #[serde(default)]
        error: Option<GeminiError>,
    },
    #[serde(other)]
    Other,
}

#[derive(Debug, Deserialize)]
struct GeminiError {
    #[serde(rename = "type", default)]
    error_type: Option<String>,
    #[serde(default)]
    message: Option<String>,
}

/// Parse a single NDJSON line from Gemini CLI's `--output-format stream-json` output.
///
/// Returns `Some(AgentOutputEvent)` for lines that produce output,
/// or `None` for lines that should be ignored.
pub fn parse_gemini_stream_line(line: &str) -> Option<AgentOutputEvent> {
    let event: GeminiStreamEvent = serde_json::from_str(line).ok()?;
    match event {
        GeminiStreamEvent::Message {
            role,
            content,
            delta,
        } => {
            if role == "assistant" && delta == Some(true) {
                Some(AgentOutputEvent::Output { text: content })
            } else {
                None
            }
        }
        GeminiStreamEvent::Result { status, error } => {
            if status == "success" {
                Some(AgentOutputEvent::Completed)
            } else {
                let error_msg = error
                    .map(|e| {
                        format!(
                            "{}: {}",
                            e.error_type.as_deref().unwrap_or("unknown"),
                            e.message.as_deref().unwrap_or("no message"),
                        )
                    })
                    .unwrap_or_else(|| format!("unknown error (status: {status})"));
                Some(AgentOutputEvent::Failed { error: error_msg })
            }
        }
        GeminiStreamEvent::Error { message } => {
            warn!("gemini non-fatal error: {message}");
            None
        }
        _ => None,
    }
}

/// Agent executor that spawns `gemini` CLI as a subprocess.
///
/// Uses `--output-format stream-json` for real-time streaming of agent output.
pub struct GeminiCliExecutor;

#[async_trait::async_trait]
impl AgentExecutor for GeminiCliExecutor {
    async fn start(&self, config: AgentConfig) -> AppResult<AgentHandle> {
        validate_config(&config)?;

        let mut cmd = Command::new("gemini");
        cmd.args(["--output-format", "stream-json"]);

        // NOTE: allowed_tools is not forwarded to Gemini CLI because the flag
        // semantics differ from Claude Code's --allowedTools. Gemini CLI tool
        // restrictions will be addressed when the flag mapping is confirmed.

        // Prompt is written to stdin (not argv) to avoid leaking via ps/proc
        // and to avoid OS argv length limits for large prompts.
        cmd.current_dir(&config.working_dir);
        cmd.stdin(Stdio::piped());
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::inherit());
        cmd.kill_on_drop(true);

        let mut child = cmd
            .spawn()
            .map_err(|e| AppError::Internal(format!("failed to spawn gemini CLI: {e}")))?;

        // Write prompt to stdin and close it so the CLI starts processing.
        if let Some(mut stdin) = child.stdin.take() {
            use tokio::io::AsyncWriteExt;
            stdin
                .write_all(config.prompt.as_bytes())
                .await
                .map_err(|e| {
                    AppError::Internal(format!("failed to write prompt to gemini stdin: {e}"))
                })?;
            // stdin is dropped here, closing the pipe
        }

        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| AppError::Internal("gemini CLI stdout not captured".into()))?;

        let cancel = CancellationToken::new();
        let (tx, rx) = mpsc::channel(256);
        let token = cancel.clone();

        let join_handle = tokio::spawn(async move {
            let reader = BufReader::new(stdout);
            let mut lines = reader.lines();

            loop {
                tokio::select! {
                    _ = token.cancelled() => {
                        let _ = child.kill().await;
                        let _ = child.wait().await; // reap to avoid zombie
                        let _ = tx.send(AgentOutputEvent::Completed).await;
                        break;
                    }
                    line = lines.next_line() => {
                        match line {
                            Ok(Some(line)) => {
                                if let Some(event) = parse_gemini_stream_line(&line) {
                                    let is_terminal = matches!(
                                        event,
                                        AgentOutputEvent::Completed | AgentOutputEvent::Failed { .. }
                                    );
                                    if tx.send(event).await.is_err() {
                                        break;
                                    }
                                    if is_terminal {
                                        break;
                                    }
                                }
                            }
                            Ok(None) => {
                                // stdout closed — check process exit code
                                let terminal = match child.wait().await {
                                    Ok(status) if status.success() => AgentOutputEvent::Completed,
                                    Ok(status) => AgentOutputEvent::Failed {
                                        error: format!("gemini exited with {status}"),
                                    },
                                    Err(e) => AgentOutputEvent::Failed {
                                        error: format!("failed to wait on gemini: {e}"),
                                    },
                                };
                                let _ = tx.send(terminal).await;
                                break;
                            }
                            Err(e) => {
                                warn!("IO error reading gemini stdout: {e}");
                                let _ = tx.send(AgentOutputEvent::Failed {
                                    error: format!("IO error: {e}"),
                                }).await;
                                break;
                            }
                        }
                    }
                }
            }
            // Reap the child on all non-wait exit paths (terminal event, IO error,
            // channel close) to prevent zombie processes.
            // Ok(None) already calls wait(); double-wait is harmless.
            let _ = child.wait().await;
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

    #[test]
    fn test_parse_assistant_delta_message() {
        let line = r#"{"type":"message","role":"assistant","content":"Hello world","delta":true,"timestamp":"2025-10-10T12:00:00.000Z"}"#;
        match parse_gemini_stream_line(line) {
            Some(AgentOutputEvent::Output { text }) => assert_eq!(text, "Hello world"),
            other => panic!("expected Output, got {other:?}"),
        }
    }

    #[test]
    fn test_parse_assistant_non_delta_ignored() {
        let line = r#"{"type":"message","role":"assistant","content":"Full response","timestamp":"2025-10-10T12:00:00.000Z"}"#;
        assert!(parse_gemini_stream_line(line).is_none());
    }

    #[test]
    fn test_parse_assistant_delta_false_ignored() {
        let line = r#"{"type":"message","role":"assistant","content":"Full","delta":false,"timestamp":"2025-10-10T12:00:00.000Z"}"#;
        assert!(parse_gemini_stream_line(line).is_none());
    }

    #[test]
    fn test_parse_user_message_ignored() {
        let line = r#"{"type":"message","role":"user","content":"List files","timestamp":"2025-10-10T12:00:00.000Z"}"#;
        assert!(parse_gemini_stream_line(line).is_none());
    }

    #[test]
    fn test_parse_result_success() {
        let line = r#"{"type":"result","status":"success","timestamp":"2025-10-10T12:00:00.000Z"}"#;
        assert!(matches!(
            parse_gemini_stream_line(line),
            Some(AgentOutputEvent::Completed)
        ));
    }

    #[test]
    fn test_parse_result_error_with_details() {
        let line = r#"{"type":"result","status":"error","error":{"type":"api_error","message":"Rate limit exceeded"},"timestamp":"2025-10-10T12:00:00.000Z"}"#;
        match parse_gemini_stream_line(line) {
            Some(AgentOutputEvent::Failed { error }) => {
                assert!(error.contains("api_error"));
                assert!(error.contains("Rate limit exceeded"));
            }
            other => panic!("expected Failed, got {other:?}"),
        }
    }

    #[test]
    fn test_parse_result_error_without_details() {
        let line = r#"{"type":"result","status":"error","timestamp":"2025-10-10T12:00:00.000Z"}"#;
        match parse_gemini_stream_line(line) {
            Some(AgentOutputEvent::Failed { error }) => {
                assert!(error.contains("error"), "should include status: {error}");
            }
            other => panic!("expected Failed, got {other:?}"),
        }
    }

    #[test]
    fn test_parse_result_error_with_partial_details() {
        let line = r#"{"type":"result","status":"error","error":{"message":"Something broke"},"timestamp":"2025-10-10T12:00:00.000Z"}"#;
        match parse_gemini_stream_line(line) {
            Some(AgentOutputEvent::Failed { error }) => {
                assert!(error.contains("Something broke"));
                assert!(
                    error.contains("unknown"),
                    "missing type should fall back: {error}"
                );
            }
            other => panic!("expected Failed, got {other:?}"),
        }
    }

    #[test]
    fn test_parse_init_ignored() {
        let line = r#"{"type":"init","timestamp":"2025-10-10T12:00:00.000Z","session_id":"abc","model":"gemini-2.0-flash"}"#;
        assert!(parse_gemini_stream_line(line).is_none());
    }

    #[test]
    fn test_parse_tool_use_ignored() {
        let line = r#"{"type":"tool_use","tool_name":"Bash","tool_id":"bash-1","parameters":{"command":"ls"},"timestamp":"2025-10-10T12:00:00.000Z"}"#;
        assert!(parse_gemini_stream_line(line).is_none());
    }

    #[test]
    fn test_parse_tool_result_ignored() {
        let line = r#"{"type":"tool_result","tool_id":"bash-1","status":"success","output":"file1.txt","timestamp":"2025-10-10T12:00:00.000Z"}"#;
        assert!(parse_gemini_stream_line(line).is_none());
    }

    #[test]
    fn test_parse_invalid_json() {
        assert!(parse_gemini_stream_line("not json").is_none());
        assert!(parse_gemini_stream_line("").is_none());
    }

    #[test]
    fn test_parse_unknown_type_ignored() {
        let line = r#"{"type":"unknown_future_event","data":"something","timestamp":"2025-10-10T12:00:00.000Z"}"#;
        assert!(parse_gemini_stream_line(line).is_none());
    }

    #[test]
    fn test_gemini_cli_executor_implements_trait() {
        let executor = GeminiCliExecutor;
        let _: &dyn AgentExecutor = &executor;
    }

    #[tokio::test]
    async fn test_gemini_cli_executor_spawn_failure() {
        use uuid::Uuid;

        use crate::models::agent_session::AgentType;

        let executor = GeminiCliExecutor;
        // Use a valid working_dir so validate_config passes,
        // but rely on `gemini` binary not being installed to trigger spawn failure.
        let config = AgentConfig {
            agent_type: AgentType::GeminiCli,
            session_id: Uuid::new_v4(),
            task_id: Uuid::new_v4(),
            working_dir: std::env::temp_dir(),
            prompt: "test".to_string(),
            allowed_tools: vec![],
        };
        match executor.start(config).await {
            Err(e) => {
                let msg = e.to_string();
                assert!(msg.contains("gemini"), "error should mention gemini: {msg}");
            }
            Ok(_) => {
                // If gemini is installed, the test passes trivially
            }
        }
    }
}
