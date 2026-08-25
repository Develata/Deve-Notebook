// apps/cli/src/server/ai_chat/stream.rs
//! plan_ref:
//!   - 16_ai_agent#native-ai-chat-runtime
//!
//! # SSE 流式请求执行器
//!
//! **功能**: 执行 OpenAI 兼容的 SSE 流式 HTTP 请求。

use super::providers::{PreparedProviderRequest, parse};
use super::settings::{ProviderProtocol, ProviderSettingsSnapshot};
use super::types::ParsedSseEvent;
use anyhow::{Result, anyhow};
use deve_core::plugin::runtime::chat_stream::{ChatStreamResponse, ChatStreamSink};
use futures::StreamExt;
use std::sync::OnceLock;
use std::time::Duration;
use tokio::time::Instant;

const NATIVE_AI_TOOL_CALLS_DISABLED_ERROR: &str =
    "Native AI Chat provider tool calls are disabled by default";
const NATIVE_AI_CONNECT_TIMEOUT: Duration = Duration::from_secs(30);
const NATIVE_AI_IDLE_TIMEOUT: Duration = Duration::from_secs(30);
const NATIVE_AI_TOTAL_TIMEOUT: Duration = Duration::from_secs(5 * 60);
const MAX_SSE_EVENT_BYTES: usize = 256 * 1024;
const MAX_SSE_WIRE_BYTES: usize = 8 * 1024 * 1024;
const MAX_NATIVE_AI_OUTPUT_BYTES: usize = 2 * 1024 * 1024;

/// 全局 HTTP 客户端单例
static HTTP_CLIENT: OnceLock<Result<reqwest::Client, String>> = OnceLock::new();

pub fn get_http_client() -> Result<&'static reqwest::Client> {
    HTTP_CLIENT
        .get_or_init(|| {
            reqwest::Client::builder()
                .pool_max_idle_per_host(5)
                .connect_timeout(NATIVE_AI_CONNECT_TIMEOUT)
                .build()
                .map_err(|err| err.to_string())
        })
        .as_ref()
        .map_err(|err| anyhow!("Failed to create HTTP client: {}", err))
}

/// 执行流式请求
pub async fn execute_stream(
    req_id: &str,
    settings: &ProviderSettingsSnapshot,
    prepared: PreparedProviderRequest,
    sink: &ChatStreamSink,
) -> Result<ChatStreamResponse> {
    let client = get_http_client()?;
    let req = build_provider_request(client, settings, &prepared);
    let total_deadline = Instant::now() + NATIVE_AI_TOTAL_TIMEOUT;
    let connect_deadline = (Instant::now() + NATIVE_AI_CONNECT_TIMEOUT).min(total_deadline);
    let response = tokio::time::timeout_at(connect_deadline, req.send())
        .await
        .map_err(|_| anyhow!("Native AI provider connect timeout"))??
        .error_for_status()
        .map_err(|error| anyhow!("Native AI provider HTTP error: {error}"))?;
    let mut stream = response.bytes_stream();
    let mut decoder = BoundedSseDecoder::default();

    let mut output = String::new();
    let mut finish_reason: Option<String> = None;
    let mut wire_bytes = 0usize;

    'stream: loop {
        let idle_deadline = (Instant::now() + NATIVE_AI_IDLE_TIMEOUT).min(total_deadline);
        let next = tokio::time::timeout_at(idle_deadline, stream.next())
            .await
            .map_err(|_| {
                if Instant::now() >= total_deadline {
                    anyhow!("Native AI provider total timeout")
                } else {
                    anyhow!("Native AI provider idle timeout")
                }
            })?;
        let Some(chunk) = next else {
            decoder.finish()?;
            break;
        };
        let chunk = chunk.map_err(|error| anyhow!("SSE stream error: {error}"))?;
        wire_bytes = wire_bytes
            .checked_add(chunk.len())
            .filter(|total| *total <= MAX_SSE_WIRE_BYTES)
            .ok_or_else(|| anyhow!("Native AI provider wire limit exceeded"))?;
        let mut apply = |data: String| {
            ensure_total_deadline(total_deadline)?;
            let step = apply_provider_data(
                req_id,
                prepared.protocol,
                &data,
                &mut output,
                &mut finish_reason,
                sink,
            )?;
            ensure_total_deadline(total_deadline)?;
            Ok(step)
        };
        if decoder.push(&chunk, &mut apply)? == StreamStep::Break {
            break 'stream;
        }
    }

    ensure_total_deadline(total_deadline)?;
    finish_stream_response(req_id, output, Vec::new(), finish_reason, sink)
}

fn ensure_total_deadline(deadline: Instant) -> Result<()> {
    if Instant::now() >= deadline {
        Err(anyhow!("Native AI provider total timeout"))
    } else {
        Ok(())
    }
}

fn apply_provider_data(
    req_id: &str,
    protocol: ProviderProtocol,
    data: &str,
    output: &mut String,
    finish_reason: &mut Option<String>,
    sink: &ChatStreamSink,
) -> Result<StreamStep> {
    if data == "[DONE]" {
        return Ok(StreamStep::Break);
    }
    apply_sse_event(
        req_id,
        parse(protocol, data).map_err(|error| anyhow!("{error}"))?,
        output,
        finish_reason,
        sink,
    )
}

#[derive(Default)]
struct BoundedSseDecoder {
    line: Vec<u8>,
    data: Vec<u8>,
    event_bytes: usize,
    has_data_field: bool,
    swallow_lf: bool,
}

impl BoundedSseDecoder {
    fn push(
        &mut self,
        chunk: &[u8],
        on_event: &mut impl FnMut(String) -> Result<StreamStep>,
    ) -> Result<StreamStep> {
        for byte in chunk {
            if self.swallow_lf {
                self.swallow_lf = false;
                if *byte == b'\n' {
                    continue;
                }
            }
            self.admit_byte()?;
            match *byte {
                b'\r' => {
                    if self.finish_line(on_event)? == StreamStep::Break {
                        return Ok(StreamStep::Break);
                    }
                    self.swallow_lf = true;
                }
                b'\n' => {
                    if self.finish_line(on_event)? == StreamStep::Break {
                        return Ok(StreamStep::Break);
                    }
                }
                byte => self.line.push(byte),
            }
        }
        Ok(StreamStep::Continue)
    }

    fn finish(self) -> Result<()> {
        if !self.line.is_empty()
            || self.has_data_field
            || !self.data.is_empty()
            || self.event_bytes != 0
        {
            return Err(anyhow!("Native AI provider SSE frame was truncated"));
        }
        Ok(())
    }

    fn admit_byte(&mut self) -> Result<()> {
        self.event_bytes = self
            .event_bytes
            .checked_add(1)
            .filter(|bytes| *bytes <= MAX_SSE_EVENT_BYTES)
            .ok_or_else(|| anyhow!("Native AI provider SSE event limit exceeded"))?;
        Ok(())
    }

    fn finish_line(
        &mut self,
        on_event: &mut impl FnMut(String) -> Result<StreamStep>,
    ) -> Result<StreamStep> {
        if self.line.is_empty() {
            let step = if self.has_data_field {
                on_event(self.take_data()?)?
            } else {
                StreamStep::Continue
            };
            self.event_bytes = 0;
            return Ok(step);
        }
        if self.line.first() == Some(&b':') {
            self.line.clear();
            return Ok(StreamStep::Continue);
        }
        let (field, mut value) = match self.line.iter().position(|byte| *byte == b':') {
            Some(colon) => (&self.line[..colon], &self.line[colon + 1..]),
            None => (&self.line[..], &[][..]),
        };
        if field == b"data" {
            if value.first() == Some(&b' ') {
                value = &value[1..];
            }
            if self.has_data_field {
                self.data.push(b'\n');
            }
            self.data.extend_from_slice(value);
            self.has_data_field = true;
        }
        self.line.clear();
        Ok(StreamStep::Continue)
    }

    fn take_data(&mut self) -> Result<String> {
        let data = std::mem::take(&mut self.data);
        self.has_data_field = false;
        String::from_utf8(data)
            .map_err(|_| anyhow!("Native AI provider SSE event is not valid UTF-8"))
    }
}

fn build_provider_request(
    client: &reqwest::Client,
    settings: &ProviderSettingsSnapshot,
    prepared: &PreparedProviderRequest,
) -> reqwest::RequestBuilder {
    let req = client.post(&prepared.endpoint).json(&prepared.body);
    match settings.provider {
        ProviderProtocol::OpenaiChatCompletions | ProviderProtocol::OpenaiResponses => {
            req.bearer_auth(&settings.api_key)
        }
        ProviderProtocol::AnthropicMessages => req
            .header("x-api-key", &settings.api_key)
            .header("anthropic-version", "2023-06-01"),
    }
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
    apply_sse_event_with_limit(
        req_id,
        event,
        output,
        finish_reason,
        sink,
        MAX_NATIVE_AI_OUTPUT_BYTES,
    )
}

fn apply_sse_event_with_limit(
    req_id: &str,
    event: ParsedSseEvent,
    output: &mut String,
    finish_reason: &mut Option<String>,
    sink: &ChatStreamSink,
    max_output_bytes: usize,
) -> Result<StreamStep> {
    match event {
        ParsedSseEvent::ContentDelta(content) => {
            output
                .len()
                .checked_add(content.len())
                .filter(|total| *total <= max_output_bytes)
                .ok_or_else(|| anyhow!("Native AI provider output limit exceeded"))?;
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
    let reason = finish_reason
        .ok_or_else(|| anyhow!("Native AI provider stream ended before a valid terminal event"))?;
    let response = finalize_stream_response(output, tool_calls)?;
    sink.send_chunk(req_id, None, Some(reason));
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
