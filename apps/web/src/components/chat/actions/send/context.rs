//! plan_ref:
//!   - 16_ai_agent#native-ai-chat-runtime
//!
//! Chat send payload helpers. This module keeps bounded context construction
//! out of the UI callback while preserving the read-first frontend boundary.

use crate::components::chat::slash_commands::ChatSessionMode;
use crate::runtime::domain::ChatMessage;

pub(super) const MAX_CHAT_CONTEXT_CHARS: usize = 16_000;
pub(super) const MAX_CHAT_HISTORY_MESSAGES: usize = 8;
const MAX_CHAT_HISTORY_CHARS: usize = 8_000;

pub(super) fn truncate_markdown_context(content: String) -> String {
    let Some((end, _)) = content.char_indices().nth(MAX_CHAT_CONTEXT_CHARS) else {
        return content;
    };
    content[..end].to_string()
}

pub(super) fn build_chat_context(
    current_doc_path: String,
    current_markdown: String,
    selection: serde_json::Value,
    session_mode: ChatSessionMode,
) -> serde_json::Value {
    serde_json::json!({
        "current_file": current_doc_path,
        "current_markdown": current_markdown,
        "selection": selection,
        "chat_mode": session_mode.as_str(),
    })
}

pub(super) fn bounded_chat_history(messages: Vec<ChatMessage>) -> Vec<serde_json::Value> {
    let mut total_chars = 0usize;
    let mut selected = Vec::new();
    for message in messages.into_iter().rev() {
        if message.content.is_empty() {
            continue;
        }
        let role = match message.role.as_str() {
            "user" | "assistant" => message.role,
            _ => continue,
        };
        let content_len = message.content.chars().count();
        if total_chars.saturating_add(content_len) > MAX_CHAT_HISTORY_CHARS {
            break;
        }
        total_chars += content_len;
        selected.push(serde_json::json!({
            "role": role,
            "content": message.content,
        }));
        if selected.len() >= MAX_CHAT_HISTORY_MESSAGES {
            break;
        }
    }
    selected.reverse();
    selected
}
