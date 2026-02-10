use crate::agent::executor::AgentOutputEvent;

/// Parse a single NDJSON line from Claude Code CLI's stream-json output.
///
/// Returns `Some(AgentOutputEvent)` for lines that produce output,
/// or `None` for lines that should be ignored (system, user, etc.).
pub fn parse_stream_line(line: &str) -> Option<AgentOutputEvent> {
    // TODO: implement
    None
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
