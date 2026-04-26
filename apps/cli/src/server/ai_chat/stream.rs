// apps/cli/src/server/ai_chat/stream.rs
//! plan_ref:
//!   - 10_ai_agent#native-ai-chat-runtime
//!
//! # SSE 流式请求执行器
//!
//! **功能**: 执行 OpenAI 兼容的 SSE 流式 HTTP 请求。

use super::sse_parser::{ToolCallBuilder, parse_sse_message};
use super::types::ParsedSseEvent;
use anyhow::{Result, anyhow};
use deve_core::plugin::runtime::chat_stream::{ChatStreamResponse, ChatStreamSink};
use futures::StreamExt;
use reqwest_eventsource::{Error as EventSourceError, Event, EventSource};
use std::collections::HashMap;
use std::sync::OnceLock;

const NATIVE_AI_TOOL_CALLS_DISABLED_ERROR: &str =
    "Native AI Chat provider tool calls are disabled by default";

/// 全局 HTTP 客户端单例
static HTTP_CLIENT: OnceLock<reqwest::Client> = OnceLock::new();

pub fn get_http_client() -> &'static reqwest::Client {
    HTTP_CLIENT.get_or_init(|| {
        reqwest::Client::builder()
            .pool_max_idle_per_host(5)
            .build()
            .expect("Failed to create HTTP client")
    })
}

/// 执行流式请求
pub async fn execute_stream(
    req_id: &str,
    endpoint: &str,
    api_key: &str,
    headers: &HashMap<String, String>,
    body: serde_json::Value,
    sink: &ChatStreamSink,
) -> Result<ChatStreamResponse> {
    let client = get_http_client();
    let mut req = client.post(endpoint).bearer_auth(api_key).json(&body);

    for (key, value) in headers {
        req = req.header(key.as_str(), value.as_str());
    }

    let mut stream =
        EventSource::new(req).map_err(|e| anyhow!("Failed to create SSE stream: {}", e))?;

    let mut output = String::new();
    let mut tool_builder = ToolCallBuilder::new();
    let mut finish_reason: Option<String> = None;

    while let Some(event) = stream.next().await {
        match event {
            Ok(Event::Open) => {}
            Ok(Event::Message(message)) => {
                if message.data == "[DONE]" {
                    finish_reason.get_or_insert_with(|| "done".to_string());
                    break;
                }

                match parse_sse_message(&message.data).map_err(|e| anyhow!("{}", e))? {
                    ParsedSseEvent::ContentDelta(content) => {
                        output.push_str(&content);
                        sink.send_chunk(req_id, Some(content), None);
                    }
                    ParsedSseEvent::ToolCallDelta {
                        index,
                        id,
                        name,
                        arguments,
                    } => {
                        tool_builder.process_delta(index, id, name, arguments);
                    }
                    ParsedSseEvent::Finished(reason) => {
                        finish_reason = Some(reason);
                        break;
                    }
                    ParsedSseEvent::Empty => {}
                }
            }
            Err(EventSourceError::StreamEnded) => break,
            Err(err) => return Err(anyhow!("SSE stream error: {}", err)),
        }
    }

    // 发送结束信号
    if let Some(reason) = finish_reason {
        sink.send_chunk(req_id, None, Some(reason));
    }

    finalize_stream_response(output, tool_builder.build())
}

fn finalize_stream_response(
    output: String,
    tool_calls: Vec<deve_core::plugin::runtime::chat_stream::ToolCallInfo>,
) -> Result<ChatStreamResponse> {
    if !tool_calls.is_empty() {
        return Err(anyhow!(NATIVE_AI_TOOL_CALLS_DISABLED_ERROR));
    }

    Ok(ChatStreamResponse::Text { content: output })
}

#[cfg(test)]
mod tests {
    use super::*;
    use deve_core::plugin::runtime::chat_stream::ToolCallInfo;

    #[test]
    fn finalize_stream_response_rejects_provider_tool_calls() {
        let err = finalize_stream_response(
            "partial".to_string(),
            vec![ToolCallInfo {
                id: "call_1".to_string(),
                name: "write_file".to_string(),
                arguments: "{}".to_string(),
            }],
        )
        .expect_err("native AI must fail closed on provider tool calls");

        assert_eq!(err.to_string(), NATIVE_AI_TOOL_CALLS_DISABLED_ERROR);
    }

    #[test]
    fn finalize_stream_response_accepts_plain_text() {
        let response = finalize_stream_response("hello".to_string(), vec![])
            .expect("plain text response should be accepted");

        match response {
            ChatStreamResponse::Text { content } => assert_eq!(content, "hello"),
            ChatStreamResponse::ToolCalls { .. } => panic!("unexpected tool response"),
        }
    }
}
