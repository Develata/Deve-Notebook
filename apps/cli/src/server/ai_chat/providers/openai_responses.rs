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
    let (instructions, input) = split_system(messages);
    let mut body = json!({
        "model": settings.model,
        "input": input,
        "stream": true,
        "max_output_tokens": settings.max_tokens,
    });
    if let Some(instructions) = instructions {
        body["instructions"] = Value::String(instructions);
    }
    body
}

pub(super) fn parse(data: &str) -> Result<ParsedSseEvent, String> {
    let event: Value = serde_json::from_str(data)
        .map_err(|error| format!("Invalid OpenAI Responses SSE payload: {error}"))?;
    let kind = event
        .get("type")
        .and_then(Value::as_str)
        .ok_or_else(|| "OpenAI Responses SSE event is missing type".to_string())?;
    if is_tool_event(kind) {
        return Ok(ParsedSseEvent::ToolCallDelta);
    }
    match kind {
        "response.output_text.delta" => event
            .get("delta")
            .and_then(Value::as_str)
            .map(|delta| ParsedSseEvent::ContentDelta(delta.to_string()))
            .ok_or_else(|| "OpenAI Responses text delta is missing delta".to_string()),
        "response.completed" => {
            validate_completed_response(&event)?;
            Ok(ParsedSseEvent::Finished("completed".to_string()))
        }
        "response.failed" | "error" => Err("OpenAI Responses provider returned an error".into()),
        "response.refusal.delta"
        | "response.refusal.done"
        | "response.function_call_arguments.delta"
        | "response.function_call_arguments.done" => Ok(ParsedSseEvent::ToolCallDelta),
        "response.output_item.added" | "response.output_item.done" => {
            classify_output_item(event.get("item"))
        }
        "response.content_part.added" | "response.content_part.done" => {
            classify_content_part(event.get("part"))
        }
        "response.created"
        | "response.in_progress"
        | "response.output_text.done"
        | "response.queued" => Ok(ParsedSseEvent::Empty),
        _ => Ok(ParsedSseEvent::Empty),
    }
}

fn is_tool_event(kind: &str) -> bool {
    const TOOL_EVENT_PREFIXES: [&str; 13] = [
        "response.function_call_arguments.",
        "response.custom_tool_call_input.",
        "response.computer_call.",
        "response.file_search_call.",
        "response.web_search_call.",
        "response.code_interpreter_call.",
        "response.image_generation_call.",
        "response.local_shell_call.",
        "response.shell_call.",
        "response.apply_patch_call.",
        "response.mcp_call.",
        "response.mcp_list_tools.",
        "response.mcp_approval_request.",
    ];
    TOOL_EVENT_PREFIXES
        .iter()
        .any(|prefix| kind.starts_with(prefix))
}

fn classify_output_item(item: Option<&Value>) -> Result<ParsedSseEvent, String> {
    match item
        .and_then(|item| item.get("type"))
        .and_then(Value::as_str)
    {
        Some("message") => Ok(ParsedSseEvent::Empty),
        Some(
            "function_call"
            | "custom_tool_call"
            | "computer_call"
            | "file_search_call"
            | "web_search_call"
            | "code_interpreter_call"
            | "image_generation_call"
            | "local_shell_call"
            | "shell_call"
            | "apply_patch_call"
            | "mcp_call"
            | "mcp_list_tools",
        ) => Ok(ParsedSseEvent::ToolCallDelta),
        Some(_) | None => Err("OpenAI Responses output item type is unsupported".to_string()),
    }
}

fn classify_content_part(part: Option<&Value>) -> Result<ParsedSseEvent, String> {
    match part
        .and_then(|part| part.get("type"))
        .and_then(Value::as_str)
    {
        Some("output_text") => {
            if part
                .and_then(|part| part.get("text"))
                .is_some_and(Value::is_string)
            {
                Ok(ParsedSseEvent::Empty)
            } else {
                Err("OpenAI Responses output_text is missing string text".to_string())
            }
        }
        Some("refusal") => Err("OpenAI Responses refusal is unsupported".to_string()),
        Some(_) | None => Err("OpenAI Responses content part type is unsupported".to_string()),
    }
}

fn validate_completed_response(event: &Value) -> Result<(), String> {
    let response = event
        .get("response")
        .and_then(Value::as_object)
        .ok_or_else(|| "OpenAI Responses completion is missing response".to_string())?;
    if response.get("status").and_then(Value::as_str) != Some("completed") {
        return Err("OpenAI Responses completion status is not completed".to_string());
    }
    let output = response
        .get("output")
        .and_then(Value::as_array)
        .ok_or_else(|| "OpenAI Responses completion output is missing".to_string())?;
    let mut saw_message = false;
    let mut saw_output_text = false;
    for item in output {
        if !matches!(classify_output_item(Some(item))?, ParsedSseEvent::Empty) {
            return Err("OpenAI Responses completion contains unsupported output item".to_string());
        }
        if item.get("type").and_then(Value::as_str) == Some("message") {
            saw_message = true;
            let content = item
                .get("content")
                .and_then(Value::as_array)
                .ok_or_else(|| "OpenAI Responses message content is missing".to_string())?;
            if content.is_empty() {
                return Err("OpenAI Responses message content is empty".to_string());
            }
            for part in content {
                classify_content_part(Some(part))?;
                if part.get("type").and_then(Value::as_str) == Some("output_text") {
                    saw_output_text = true;
                }
            }
        }
    }
    if !saw_message {
        return Err("OpenAI Responses completion is missing message output".to_string());
    }
    if !saw_output_text {
        return Err("OpenAI Responses completion is missing output_text".to_string());
    }
    Ok(())
}
