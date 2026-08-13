// apps/cli/src/server/ai_chat/sse_parser.rs
//! plan_ref:
//!   - 16_ai_agent#native-ai-chat-runtime
//!
//! # SSE 消息解析器
//!
//! **功能**: 解析 OpenAI 兼容的 SSE 流式响应。
//!
//! **Pre-condition**: 输入为有效的 JSON 字符串。
//! **Post-condition**: 返回结构化的 SSE 事件或错误。

use super::types::ParsedSseEvent;
use serde_json::Value;

/// 解析单条 SSE 消息
pub fn parse_sse_message(data: &str) -> Result<ParsedSseEvent, String> {
    let response: Value =
        serde_json::from_str(data).map_err(|e| format!("Invalid SSE payload: {e}"))?;
    let choices = response
        .get("choices")
        .and_then(Value::as_array)
        .ok_or_else(|| "Missing choices in SSE payload".to_string())?;
    if choices.iter().any(choice_has_tool_call_signal) {
        return Ok(ParsedSseEvent::ToolCallDelta);
    }
    if choices.len() != 1 {
        return Err("OpenAI Chat SSE must contain exactly one choice".to_string());
    }
    let choice = choices[0]
        .as_object()
        .ok_or_else(|| "OpenAI Chat choice must be an object".to_string())?;
    if let Some(reason) = choice.get("finish_reason").filter(|value| !value.is_null()) {
        return match reason.as_str() {
            Some("stop") => Ok(ParsedSseEvent::Finished("stop".to_string())),
            Some("tool_calls" | "function_call") => Ok(ParsedSseEvent::ToolCallDelta),
            Some(_) => Err("OpenAI Chat finish reason is unsupported".to_string()),
            None => Err("OpenAI Chat finish reason must be a string".to_string()),
        };
    }
    let Some(delta) = choice.get("delta").filter(|value| !value.is_null()) else {
        return Ok(ParsedSseEvent::Empty);
    };
    let delta = delta
        .as_object()
        .ok_or_else(|| "OpenAI Chat delta must be an object".to_string())?;
    const ALLOWED_DELTA_FIELDS: [&str; 5] =
        ["role", "content", "refusal", "function_call", "tool_calls"];
    if delta
        .keys()
        .any(|key| !ALLOWED_DELTA_FIELDS.contains(&key.as_str()))
    {
        return Err("OpenAI Chat delta contains an unsupported field".to_string());
    }
    if delta.get("refusal").is_some_and(|value| !value.is_null()) {
        return Err("OpenAI Chat refusal is unsupported".to_string());
    }
    if delta
        .get("function_call")
        .is_some_and(|value| !value.is_null())
        || delta
            .get("tool_calls")
            .is_some_and(|value| !value.is_null())
    {
        return Ok(ParsedSseEvent::ToolCallDelta);
    }
    if delta
        .get("role")
        .filter(|value| !value.is_null())
        .is_some_and(|role| role.as_str() != Some("assistant"))
    {
        return Err("OpenAI Chat delta role is unsupported".to_string());
    }
    match delta.get("content").filter(|value| !value.is_null()) {
        Some(content) => content
            .as_str()
            .map(|content| ParsedSseEvent::ContentDelta(content.to_string()))
            .ok_or_else(|| "OpenAI Chat content delta must be text".to_string()),
        None => Ok(ParsedSseEvent::Empty),
    }
}

fn choice_has_tool_call_signal(choice: &Value) -> bool {
    matches!(
        choice.get("finish_reason").and_then(Value::as_str),
        Some("tool_calls" | "function_call")
    ) || choice.get("delta").is_some_and(|delta| {
        delta
            .get("function_call")
            .is_some_and(|value| !value.is_null())
            || delta
                .get("tool_calls")
                .is_some_and(|value| !value.is_null())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_content_delta() {
        let data = r#"{"choices":[{"delta":{"content":"Hello"}}]}"#;
        let event = parse_sse_message(data).unwrap();
        match event {
            ParsedSseEvent::ContentDelta(content) => assert_eq!(content, "Hello"),
            _ => panic!("Expected ContentDelta"),
        }
    }

    #[test]
    fn test_parse_finish_reason() {
        let data = r#"{"choices":[{"finish_reason":"stop","delta":{}}]}"#;
        let event = parse_sse_message(data).unwrap();
        match event {
            ParsedSseEvent::Finished(reason) => assert_eq!(reason, "stop"),
            _ => panic!("Expected Finished"),
        }
    }

    #[test]
    fn chat_refusal_filter_and_unknown_delta_fail_closed() {
        assert!(parse_sse_message(r#"{"choices":[{"delta":{"refusal":"no"}}]}"#).is_err());
        assert!(
            parse_sse_message(r#"{"choices":[{"finish_reason":"content_filter","delta":{}}]}"#)
                .is_err()
        );
        assert!(parse_sse_message(r#"{"choices":[{"delta":{"audio":{"id":"x"}}}]}"#).is_err());
    }

    #[test]
    fn test_parse_tool_call_delta() {
        let data = r#"{"choices":[{"delta":{"tool_calls":[{"index":0,"id":"call_1","function":{"name":"get_time","arguments":"{}"}}]}}]}"#;
        let event = parse_sse_message(data).unwrap();
        assert!(matches!(event, ParsedSseEvent::ToolCallDelta));
    }

    #[test]
    fn test_parse_tool_call_finish_reason_as_tool_call_signal() {
        let data = r#"{"choices":[{"finish_reason":"tool_calls","delta":{}}]}"#;
        let event = parse_sse_message(data).unwrap();
        assert!(matches!(event, ParsedSseEvent::ToolCallDelta));
    }

    #[test]
    fn test_parse_legacy_function_call_delta_as_tool_call_signal() {
        let data =
            r#"{"choices":[{"delta":{"function_call":{"name":"write_file","arguments":"{}"}}}]}"#;
        let event = parse_sse_message(data).unwrap();
        assert!(matches!(event, ParsedSseEvent::ToolCallDelta));
    }

    #[test]
    fn test_parse_legacy_function_call_finish_reason_as_tool_call_signal() {
        let data = r#"{"choices":[{"finish_reason":"function_call","delta":{}}]}"#;
        let event = parse_sse_message(data).unwrap();
        assert!(matches!(event, ParsedSseEvent::ToolCallDelta));
    }

    #[test]
    fn test_parse_tool_calls_take_priority_over_content() {
        let data =
            r#"{"choices":[{"delta":{"content":"unsafe partial","tool_calls":[{"index":0}]}}]}"#;
        let event = parse_sse_message(data).unwrap();
        assert!(matches!(event, ParsedSseEvent::ToolCallDelta));
    }

    #[test]
    fn test_parse_tool_calls_take_priority_over_finish_reason() {
        let data = r#"{"choices":[{"finish_reason":"stop","delta":{"tool_calls":[{"index":0}]}}]}"#;
        let event = parse_sse_message(data).unwrap();
        assert!(matches!(event, ParsedSseEvent::ToolCallDelta));
    }

    #[test]
    fn test_parse_tool_calls_from_any_choice_fail_closed() {
        let data =
            r#"{"choices":[{"delta":{"content":"safe"}},{"delta":{"tool_calls":[{"index":0}]}}]}"#;
        let event = parse_sse_message(data).unwrap();
        assert!(matches!(event, ParsedSseEvent::ToolCallDelta));
    }

    #[test]
    fn test_parse_invalid_json() {
        let data = "not json";
        let result = parse_sse_message(data);
        assert!(result.is_err());
    }
}
