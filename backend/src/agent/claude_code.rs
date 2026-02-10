use std::process::Stdio;

use serde::Deserialize;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;
use tracing::warn;

use crate::agent::executor::{AgentConfig, AgentExecutor, AgentHandle, AgentOutputEvent};
use crate::error::{AppError, AppResult};

/// Top-level message types from Claude Code CLI's `--output-format=stream-json`.
#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
#[serde(rename_all = "snake_case")]
enum StreamMessage {
    System {},
    Assistant {},
    User {},
    Result { is_error: bool, subtype: String },
    StreamEvent { event: RawStreamEvent },
}

/// Raw streaming event from the Claude API, wrapped in a `stream_event` message.
#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
#[serde(rename_all = "snake_case")]
enum RawStreamEvent {
    ContentBlockDelta {
        delta: Delta,
    },
    #[serde(other)]
    Other,
}

/// Delta payload within a `content_block_delta` event.
#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
#[serde(rename_all = "snake_case")]
enum Delta {
    TextDelta {
        text: String,
    },
    #[serde(other)]
    Other,
}

/// Parse a single NDJSON line from Claude Code CLI's stream-json output.
///
/// Returns `Some(AgentOutputEvent)` for lines that produce output,
/// or `None` for lines that should be ignored (system, user, etc.).
pub fn parse_stream_line(line: &str) -> Option<AgentOutputEvent> {
    let msg: StreamMessage = serde_json::from_str(line).ok()?;
    match msg {
        StreamMessage::StreamEvent {
            event:
                RawStreamEvent::ContentBlockDelta {
                    delta: Delta::TextDelta { text },
                },
        } => Some(AgentOutputEvent::Output { text }),
        StreamMessage::Result {
            is_error: false, ..
        } => Some(AgentOutputEvent::Completed),
        StreamMessage::Result {
            is_error: true,
            subtype,
        } => Some(AgentOutputEvent::Failed { error: subtype }),
        _ => None,
    }
}

/// Agent executor that spawns `claude` CLI as a subprocess.
///
/// Uses `--output-format=stream-json --include-partial-messages` for
/// real-time token-level streaming of agent output.
pub struct ClaudeCodeExecutor;

#[async_trait::async_trait]
impl AgentExecutor for ClaudeCodeExecutor {
    async fn start(&self, config: AgentConfig) -> AppResult<AgentHandle> {
        let mut cmd = Command::new("claude");
        cmd.args([
            "-p",
            "--output-format=stream-json",
            "--include-partial-messages",
        ]);

        if !config.allowed_tools.is_empty() {
            cmd.arg("--allowedTools");
            for tool in &config.allowed_tools {
                cmd.arg(tool);
            }
        }

        // Prompt is written to stdin (not argv) to avoid leaking via ps/proc
        // and to avoid OS argv length limits for large prompts.
        cmd.current_dir(&config.working_dir);
        cmd.stdin(Stdio::piped());
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::inherit());
        cmd.kill_on_drop(true);

        let mut child = cmd
            .spawn()
            .map_err(|e| AppError::Internal(format!("failed to spawn claude CLI: {e}")))?;

        // Write prompt to stdin and close it so the CLI starts processing.
        if let Some(mut stdin) = child.stdin.take() {
            use tokio::io::AsyncWriteExt;
            stdin
                .write_all(config.prompt.as_bytes())
                .await
                .map_err(|e| {
                    AppError::Internal(format!("failed to write prompt to claude stdin: {e}"))
                })?;
            // stdin is dropped here, closing the pipe
        }

        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| AppError::Internal("claude CLI stdout not captured".into()))?;

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
                        let _ = tx.send(AgentOutputEvent::Completed).await;
                        break;
                    }
                    line = lines.next_line() => {
                        match line {
                            Ok(Some(line)) => {
                                if let Some(event) = parse_stream_line(&line) {
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
                                        error: format!("claude exited with {status}"),
                                    },
                                    Err(e) => AgentOutputEvent::Failed {
                                        error: format!("failed to wait on claude: {e}"),
                                    },
                                };
                                let _ = tx.send(terminal).await;
                                break;
                            }
                            Err(e) => {
                                warn!("IO error reading claude stdout: {e}");
                                let _ = tx.send(AgentOutputEvent::Failed {
                                    error: format!("IO error: {e}"),
                                }).await;
                                break;
                            }
                        }
                    }
                }
            }
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
    fn test_parse_text_delta() {
        let line = r#"{"type":"stream_event","event":{"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"Hello world"}}}"#;
        let event = parse_stream_line(line);
        assert!(event.is_some(), "text_delta should produce an event");
        match event.unwrap() {
            AgentOutputEvent::Output { text } => assert_eq!(text, "Hello world"),
            other => panic!("expected Output, got {other:?}"),
        }
    }

    #[test]
    fn test_parse_result_success() {
        let line = r#"{"type":"result","subtype":"success","is_error":false,"total_cost_usd":0.05,"usage":{}}"#;
        let event = parse_stream_line(line);
        assert!(event.is_some(), "result success should produce an event");
        assert!(
            matches!(event.unwrap(), AgentOutputEvent::Completed),
            "result success should map to Completed"
        );
    }

    #[test]
    fn test_parse_result_error() {
        let line = r#"{"type":"result","subtype":"error_during_execution","is_error":true,"total_cost_usd":0.01,"usage":{}}"#;
        let event = parse_stream_line(line);
        assert!(event.is_some(), "result error should produce an event");
        match event.unwrap() {
            AgentOutputEvent::Failed { error } => {
                assert_eq!(error, "error_during_execution");
            }
            other => panic!("expected Failed, got {other:?}"),
        }
    }

    #[test]
    fn test_parse_system_ignored() {
        let line = r#"{"type":"system","subtype":"init","session_id":"abc-123","model":"claude-sonnet-4-5-20250929","tools":["Read","Write"]}"#;
        assert!(
            parse_stream_line(line).is_none(),
            "system messages should be ignored"
        );
    }

    #[test]
    fn test_parse_assistant_ignored() {
        let line = r#"{"type":"assistant","message":{"content":[{"type":"text","text":"Hello"}]}}"#;
        assert!(
            parse_stream_line(line).is_none(),
            "assistant messages should be ignored (stream_event handles streaming)"
        );
    }

    #[test]
    fn test_parse_invalid_json() {
        assert!(
            parse_stream_line("not valid json {{{").is_none(),
            "invalid JSON should return None"
        );
        assert!(
            parse_stream_line("").is_none(),
            "empty line should return None"
        );
    }

    #[test]
    fn test_parse_input_json_delta_ignored() {
        let line = r#"{"type":"stream_event","event":{"type":"content_block_delta","index":1,"delta":{"type":"input_json_delta","partial_json":"{\"file\":"}}}"#;
        assert!(
            parse_stream_line(line).is_none(),
            "input_json_delta (tool use) should be ignored"
        );
    }

    #[test]
    fn test_parse_message_stop_ignored() {
        let line = r#"{"type":"stream_event","event":{"type":"message_stop"}}"#;
        assert!(
            parse_stream_line(line).is_none(),
            "message_stop event should be ignored"
        );
    }
}
