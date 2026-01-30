// apps/cli/src/server/ai_chat/sse_parser.rs
//! # SSE 消息解析器
//!
//! **功能**: 解析 OpenAI 兼容的 SSE 流式响应。
//!
//! **Invariant**: 工具调用按 index 顺序累积，index 必须单调递增。
//! **Pre-condition**: 输入为有效的 JSON 字符串。
//! **Post-condition**: 返回结构化的 SSE 事件或错误。

use super::types::{ParsedSseEvent, SseResponse};
use deve_core::plugin::runtime::chat_stream::ToolCallInfo;

/// 工具调用构建器 (状态机)
///
/// **Invariant**: `calls` 按 index 顺序存储，index 对应数组下标。
#[derive(Debug, Default)]
pub struct ToolCallBuilder {
    calls: Vec<(String, String, String)>, // (id, name, arguments)
}

impl ToolCallBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    /// 处理工具调用增量
    ///
    /// **Invariant**: index 必须 <= calls.len()
    pub fn process_delta(
        &mut self,
        index: usize,
        id: Option<String>,
        name: Option<String>,
        arguments: Option<String>,
    ) {
        // 扩展数组以容纳新的 index
        while self.calls.len() <= index {
            self.calls
                .push((String::new(), String::new(), String::new()));
        }

        let entry = &mut self.calls[index];

        if let Some(id) = id {
            entry.0 = id;
        }
        if let Some(name) = name {
            entry.1 = name;
        }
        if let Some(args) = arguments {
            entry.2.push_str(&args);
        }
    }

    /// 构建最终的工具调用列表
    pub fn build(self) -> Vec<ToolCallInfo> {
        self.calls
            .into_iter()
            .filter(|(id, name, _)| !id.is_empty() && !name.is_empty())
            .map(|(id, name, arguments)| ToolCallInfo {
                id,
                name,
                arguments,
            })
            .collect()
    }
}

/// 解析单条 SSE 消息
pub fn parse_sse_message(data: &str) -> Result<ParsedSseEvent, String> {
    let response: SseResponse =
        serde_json::from_str(data).map_err(|e| format!("Invalid SSE payload: {}", e))?;

    let choice = response
        .choices
        .first()
        .ok_or_else(|| "Missing choices in SSE payload".to_string())?;

    // 优先检查 finish_reason
    if let Some(reason) = &choice.finish_reason {
        return Ok(ParsedSseEvent::Finished(reason.clone()));
    }

    let Some(delta) = &choice.delta else {
        return Ok(ParsedSseEvent::Empty);
    };

    // 处理文本内容
    if let Some(content) = &delta.content
        && !content.is_empty()
    {
        return Ok(ParsedSseEvent::ContentDelta(content.clone()));
    }

    // 处理工具调用
    if let Some(tool_calls) = &delta.tool_calls
        && let Some(tc) = tool_calls.first()
    {
        return Ok(ParsedSseEvent::ToolCallDelta {
            index: tc.index.unwrap_or(0),
            id: tc.id.clone(),
            name: tc.function.as_ref().and_then(|f| f.name.clone()),
            arguments: tc.function.as_ref().and_then(|f| f.arguments.clone()),
        });
    }

    Ok(ParsedSseEvent::Empty)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_call_builder_single_call() {
        let mut builder = ToolCallBuilder::new();
        builder.process_delta(0, Some("call_1".into()), Some("get_weather".into()), None);
        builder.process_delta(0, None, None, Some(r#"{"city":"#.into()));
        builder.process_delta(0, None, None, Some(r#""NYC"}"#.into()));

        let calls = builder.build();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].id, "call_1");
        assert_eq!(calls[0].name, "get_weather");
        assert_eq!(calls[0].arguments, r#"{"city":"NYC"}"#);
    }

    #[test]
    fn test_tool_call_builder_multiple_calls() {
        let mut builder = ToolCallBuilder::new();
        // 第一个工具调用
        builder.process_delta(0, Some("call_1".into()), Some("read_file".into()), None);
        builder.process_delta(0, None, None, Some(r#"{"path":"a.rs"}"#.into()));
        // 第二个工具调用 (index=1)
        builder.process_delta(1, Some("call_2".into()), Some("write_file".into()), None);
        builder.process_delta(1, None, None, Some(r#"{"path":"b.rs"}"#.into()));

        let calls = builder.build();
        assert_eq!(calls.len(), 2);
        assert_eq!(calls[0].name, "read_file");
        assert_eq!(calls[1].name, "write_file");
    }

    #[test]
    fn test_tool_call_builder_sparse_index() {
        // 测试跳跃的 index（虽然不常见，但应该处理）
        let mut builder = ToolCallBuilder::new();
        builder.process_delta(
            2,
            Some("call_3".into()),
            Some("func".into()),
            Some("{}".into()),
        );

        let calls = builder.build();
        // index 0, 1 为空，应被过滤
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].id, "call_3");
    }

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
        match event {
            ParsedSseEvent::ToolCallDelta {
                index,
                id,
                name,
                arguments,
            } => {
                assert_eq!(index, 0);
                assert_eq!(id, Some("call_1".to_string()));
                assert_eq!(name, Some("get_time".to_string()));
                assert_eq!(arguments, Some("{}".to_string()));
            }
            _ => panic!("Expected ToolCallDelta"),
        }
    }

    #[test]
    fn test_parse_invalid_json() {
        let data = "not json";
        let result = parse_sse_message(data);
        assert!(result.is_err());
    }
}
