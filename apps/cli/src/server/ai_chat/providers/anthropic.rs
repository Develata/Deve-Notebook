//! plan_ref:
//!   - 16_ai_agent#native-ai-chat-runtime

use super::{HistoryMessage, split_system};
use crate::server::ai_chat::settings::ProviderSettingsSnapshot;
use crate::server::ai_chat::types::ParsedSseEvent;
use serde_json::{Value, json};

pub(super) fn request_body(
    settings: &ProviderSettingsSnapshot,
    messages: &[HistoryMessage],
) -> Value {
    let (system, messages) = split_system(messages);
    let mut body = json!({
        "model": settings.model,
        "messages": messages,
        "stream": true,
        "max_tokens": settings.max_tokens,
    });
    if let Some(system) = system {
        body["system"] = Value::String(system);
    }
    body
}

pub(super) fn parse(data: &str) -> Result<ParsedSseEvent, String> {
    let event: Value = serde_json::from_str(data)
        .map_err(|error| format!("Invalid Anthropic SSE payload: {error}"))?;
    let kind = event
        .get("type")
        .and_then(Value::as_str)
        .ok_or_else(|| "Anthropic SSE event is missing type".to_string())?;
    match kind {
        "content_block_delta" => match event
            .get("delta")
            .and_then(|delta| delta.get("type"))
            .and_then(Value::as_str)
        {
            Some("text_delta") => event
                .get("delta")
                .and_then(|delta| delta.get("text"))
                .and_then(Value::as_str)
                .map(|text| ParsedSseEvent::ContentDelta(text.to_string()))
                .ok_or_else(|| "Anthropic text delta is missing text".to_string()),
            Some("input_json_delta") => Ok(ParsedSseEvent::ToolCallDelta),
            Some("thinking_delta" | "signature_delta") => {
                Err("Anthropic thinking content is unsupported".into())
            }
            Some(_) | None => Err("Anthropic content delta type is unsupported".into()),
        },
        "content_block_start" => match event
            .get("content_block")
            .and_then(|block| block.get("type"))
            .and_then(Value::as_str)
        {
            Some("text") => Ok(ParsedSseEvent::Empty),
            Some("tool_use" | "server_tool_use") => Ok(ParsedSseEvent::ToolCallDelta),
            Some(_) | None => Err("Anthropic content block type is unsupported".into()),
        },
        "message_delta"
            if event
                .get("delta")
                .and_then(|delta| delta.get("stop_reason"))
                .and_then(Value::as_str)
                .is_some_and(|reason| matches!(reason, "tool_use" | "server_tool_use")) =>
        {
            Ok(ParsedSseEvent::ToolCallDelta)
        }
        "message_delta" => match event
            .get("delta")
            .and_then(|delta| delta.get("stop_reason"))
            .and_then(Value::as_str)
        {
            Some("end_turn" | "max_tokens" | "stop_sequence") => event
                .get("delta")
                .and_then(|delta| delta.get("stop_reason"))
                .and_then(Value::as_str)
                .map(|reason| ParsedSseEvent::Finished(reason.to_string()))
                .ok_or_else(|| "Anthropic stop reason is missing".to_string()),
            Some("tool_use" | "server_tool_use") => Ok(ParsedSseEvent::ToolCallDelta),
            Some("refusal") => Err("Anthropic refusal is unsupported".into()),
            Some(_) => Err("Anthropic stop reason is unsupported".into()),
            None => Ok(ParsedSseEvent::Empty),
        },
        "error" => Err("Anthropic provider returned an error".into()),
        "message_start" | "content_block_stop" | "message_stop" | "ping" => {
            Ok(ParsedSseEvent::Empty)
        }
        _ => Ok(ParsedSseEvent::Empty),
    }
}
