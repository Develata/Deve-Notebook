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

use super::types::{ParsedSseEvent, SseResponse};

/// 解析单条 SSE 消息
pub fn parse_sse_message(data: &str) -> Result<ParsedSseEvent, String> {
    let response: SseResponse =
        serde_json::from_str(data).map_err(|e| format!("Invalid SSE payload: {}", e))?;

    let choice = response
        .choices
        .first()
        .ok_or_else(|| "Missing choices in SSE payload".to_string())?;

    // Native AI Chat 不支持工具调用；任何工具调用信号都必须优先拒绝。
    if response.choices.iter().any(choice_has_tool_call_signal) {
        return Ok(ParsedSseEvent::ToolCallDelta);
    }

    let Some(delta) = &choice.delta else {
        if let Some(reason) = &choice.finish_reason {
            return Ok(ParsedSseEvent::Finished(reason.clone()));
        }
        return Ok(ParsedSseEvent::Empty);
    };

    if let Some(reason) = &choice.finish_reason {
        return Ok(ParsedSseEvent::Finished(reason.clone()));
    }

    // 处理文本内容
    if let Some(content) = &delta.content
        && !content.is_empty()
    {
        return Ok(ParsedSseEvent::ContentDelta(content.clone()));
    }

    Ok(ParsedSseEvent::Empty)
}

fn choice_has_tool_call_signal(choice: &super::types::SseChoice) -> bool {
    if matches!(
        choice.finish_reason.as_deref(),
        Some("tool_calls" | "function_call")
    ) {
        return true;
    }

    let Some(delta) = choice.delta.as_ref() else {
        return false;
    };

    if delta.function_call.is_some() {
        return true;
    }

    delta
        .tool_calls
        .as_ref()
        .is_some_and(|tool_calls| !tool_calls.is_empty())
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
