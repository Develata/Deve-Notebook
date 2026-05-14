// apps/cli/src/server/ai_chat/stream.rs
//! plan_ref:
//!   - 10_ai_agent#native-ai-chat-runtime
//!
//! # SSE 流式请求执行器
//!
//! **功能**: 执行 OpenAI 兼容的 SSE 流式 HTTP 请求。

use super::sse_parser::parse_sse_message;
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
static HTTP_CLIENT: OnceLock<Result<reqwest::Client, String>> = OnceLock::new();

pub fn get_http_client() -> Result<&'static reqwest::Client> {
    HTTP_CLIENT
        .get_or_init(|| {
            reqwest::Client::builder()
                .pool_max_idle_per_host(5)
                .build()
                .map_err(|err| err.to_string())
        })
        .as_ref()
        .map_err(|err| anyhow!("Failed to create HTTP client: {}", err))
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
    let client = get_http_client()?;
    let mut req = client.post(endpoint).bearer_auth(api_key).json(&body);

    for (key, value) in headers {
        req = req.header(key.as_str(), value.as_str());
    }

    let mut stream =
        EventSource::new(req).map_err(|e| anyhow!("Failed to create SSE stream: {}", e))?;

    let mut output = String::new();
    let mut finish_reason: Option<String> = None;

    while let Some(event) = stream.next().await {
        match event {
            Ok(Event::Open) => {}
            Ok(Event::Message(message)) => {
                if message.data == "[DONE]" {
                    finish_reason.get_or_insert_with(|| "done".to_string());
                    break;
                }

                if apply_sse_event(
                    req_id,
                    parse_sse_message(&message.data).map_err(|e| anyhow!("{}", e))?,
                    &mut output,
                    &mut finish_reason,
                    sink,
                )? == StreamStep::Break
                {
                    break;
                }
            }
            Err(EventSourceError::StreamEnded) => break,
            Err(err) => return Err(anyhow!("SSE stream error: {}", err)),
        }
    }

    finish_stream_response(req_id, output, Vec::new(), finish_reason, sink)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum StreamStep {
    Continue,
    Break,
}

fn apply_sse_event(
    req_id: &str,
    event: ParsedSseEvent,
    output: &mut String,
    finish_reason: &mut Option<String>,
    sink: &ChatStreamSink,
) -> Result<StreamStep> {
    match event {
        ParsedSseEvent::ContentDelta(content) => {
            output.push_str(&content);
            sink.send_chunk(req_id, Some(content), None);
            Ok(StreamStep::Continue)
        }
        ParsedSseEvent::ToolCallDelta => Err(anyhow!(NATIVE_AI_TOOL_CALLS_DISABLED_ERROR)),
        ParsedSseEvent::Finished(reason) => {
            *finish_reason = Some(reason);
            Ok(StreamStep::Break)
        }
        ParsedSseEvent::Empty => Ok(StreamStep::Continue),
    }
}

fn finish_stream_response(
    req_id: &str,
    output: String,
    tool_calls: Vec<deve_core::plugin::runtime::chat_stream::ToolCallInfo>,
    finish_reason: Option<String>,
    sink: &ChatStreamSink,
) -> Result<ChatStreamResponse> {
    let response = finalize_stream_response(output, tool_calls)?;
    if let Some(reason) = finish_reason {
        sink.send_chunk(req_id, None, Some(reason));
    }
    Ok(response)
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
mod tests;
