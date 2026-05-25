//! plan_ref:
//!   - 16_ai_agent#native-ai-chat-runtime
//!   - 10_rendering#document-authority-bridge
//!
use super::send_backend::{ChatMessagePlan, ChatSendRuntimePlan, plan_chat_send_runtime};
use crate::api::{fetch_ai_backend_capabilities, resolve_backend_for_send};
use crate::editor::ffi::{getEditorContent, try_get_editor_selection};
use crate::hooks::use_core::{ChatMessage, CoreState};
use leptos::prelude::*;
use leptos::task::spawn_local;

use crate::components::chat::slash_commands::{ChatSessionMode, consume_slash_command};

const MAX_CHAT_CONTEXT_CHARS: usize = 16_000;
const MAX_CHAT_HISTORY_MESSAGES: usize = 8;
const MAX_CHAT_HISTORY_CHARS: usize = 8_000;

pub fn make_send_text(
    core: CoreState,
    is_streaming: ReadSignal<bool>,
    session_mode: ReadSignal<ChatSessionMode>,
    set_session_mode: WriteSignal<ChatSessionMode>,
    on_req_id: Option<Callback<String>>,
    on_user_text: Option<Callback<String>>,
    on_mode_change: Option<Callback<ChatSessionMode>>,
) -> Callback<String> {
    Callback::new(move |msg: String| {
        let msg = msg.trim().to_string();
        if msg.is_empty() || is_streaming.get() {
            return;
        }
        if let Some(command) = consume_slash_command(&msg, session_mode.get_untracked()) {
            debug_assert!(!command.send_plugin_call);
            debug_assert!(!command.change_backend);
            set_session_mode.set(command.next_mode);
            if let Some(cb) = on_mode_change.as_ref() {
                cb.run(command.next_mode);
            }
            return;
        }
        core.set_is_chat_streaming.set(true);
        let req_id = uuid::Uuid::new_v4().to_string();
        let history = bounded_chat_history(core.chat_messages.get_untracked());
        let core_for_send = core.clone();
        let on_user_text = on_user_text.clone();
        let on_req_id = on_req_id.clone();
        spawn_local(async move {
            let cap = fetch_ai_backend_capabilities().await;
            let decision =
                resolve_backend_for_send(core_for_send.ai_mode.get_untracked().as_str(), &cap);
            let ChatSendRuntimePlan {
                plugin_id,
                switch_backend,
                messages,
                register_pending_req,
                stop_streaming,
            } = plan_chat_send_runtime(decision);
            if let Some(backend) = switch_backend {
                core_for_send.set_ai_mode.set(backend.to_string());
            }
            for message in messages {
                append_planned_chat_message(&core_for_send, &msg, &req_id, message);
            }
            if let Some(cb) = on_user_text.as_ref() {
                cb.run(msg.clone());
            }
            if stop_streaming {
                core_for_send.set_is_chat_streaming.set(false);
                return;
            };
            if register_pending_req && let Some(cb) = on_req_id.as_ref() {
                cb.run(req_id.clone());
            }
            let Some(plugin_id) = plugin_id else {
                return;
            };
            let current_doc_path = core_for_send
                .current_doc
                .get_untracked()
                .and_then(|doc_id| {
                    core_for_send
                        .docs
                        .get_untracked()
                        .iter()
                        .find(|(id, _)| *id == doc_id)
                        .map(|(_, path)| path.clone())
                })
                .unwrap_or_default();
            let current_markdown = if current_doc_path.is_empty() {
                String::new()
            } else {
                truncate_markdown_context(getEditorContent())
            };
            let sel_json = try_get_editor_selection().unwrap_or_else(|| "null".to_string());
            let selection = serde_json::from_str(&sel_json).unwrap_or(serde_json::Value::Null);
            let context = build_chat_context(
                current_doc_path,
                current_markdown,
                selection,
                session_mode.get_untracked(),
            );
            let args = vec![
                serde_json::json!(req_id),
                serde_json::json!(msg),
                context,
                serde_json::json!(history),
            ];
            core_for_send.on_plugin_call.run((
                req_id,
                plugin_id.to_string(),
                "chat".to_string(),
                args,
            ));
        });
    })
}

fn append_planned_chat_message(
    core: &CoreState,
    msg: &str,
    req_id: &str,
    message: ChatMessagePlan,
) {
    match message {
        ChatMessagePlan::UserInput => core.append_chat_message("user", msg, None),
        ChatMessagePlan::AssistantNotice(notice) => {
            core.append_chat_message("assistant", &notice, None);
        }
        ChatMessagePlan::AssistantPlaceholder => {
            core.append_chat_message("assistant", "", Some(req_id.to_string()));
        }
        ChatMessagePlan::AssistantError(reason) => {
            core.append_chat_message("assistant", &reason, None);
        }
    }
}

fn truncate_markdown_context(content: String) -> String {
    let Some((end, _)) = content.char_indices().nth(MAX_CHAT_CONTEXT_CHARS) else {
        return content;
    };
    content[..end].to_string()
}

fn build_chat_context(
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

fn bounded_chat_history(messages: Vec<ChatMessage>) -> Vec<serde_json::Value> {
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

pub fn make_send_example(
    send_text: Callback<String>,
    set_input: WriteSignal<String>,
) -> Callback<String> {
    Callback::new(move |example: String| {
        set_input.set(String::new());
        send_text.run(example);
    })
}

pub fn make_send_message(
    input: ReadSignal<String>,
    set_input: WriteSignal<String>,
    is_streaming: ReadSignal<bool>,
    send_text: Callback<String>,
) -> Callback<()> {
    Callback::new(move |_| {
        let msg = input.get().trim().to_string();
        if msg.is_empty() || is_streaming.get() {
            return;
        }
        set_input.set(String::new());
        send_text.run(msg);
    })
}

#[cfg(test)]
mod tests;
