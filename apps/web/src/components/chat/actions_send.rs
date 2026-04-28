//! plan_ref:
//!   - 10_ai_agent#native-ai-chat-runtime
//!   - 03_rendering#document-authority-bridge
//!
use crate::editor::ffi::{getEditorContent, try_get_editor_selection};
use crate::hooks::use_core::CoreState;
use leptos::prelude::*;

use crate::components::chat::slash_commands::{ChatSessionMode, consume_slash_command};

const MAX_CHAT_CONTEXT_CHARS: usize = 16_000;

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
            set_session_mode.set(command.next_mode);
            if let Some(cb) = on_mode_change.as_ref() {
                cb.run(command.next_mode);
            }
            return;
        }
        let req_id = uuid::Uuid::new_v4().to_string();
        core.append_chat_message("user", &msg, None);
        core.append_chat_message("assistant", "", Some(req_id.clone()));
        core.set_is_chat_streaming.set(true);
        if let Some(cb) = on_user_text.as_ref() {
            cb.run(msg.clone());
        }
        if let Some(cb) = on_req_id.as_ref() {
            cb.run(req_id.clone());
        }
        let current_doc_path = core
            .current_doc
            .get_untracked()
            .and_then(|doc_id| {
                core.docs
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
        let context = serde_json::json!({
            "current_file": current_doc_path,
            "current_markdown": current_markdown,
            "selection": selection,
            "chat_mode": session_mode.get_untracked().as_str(),
        });
        let args = vec![serde_json::json!(req_id), serde_json::json!(msg), context];
        let plugin_id = core.ai_mode.get_untracked();
        core.on_plugin_call
            .run((req_id, plugin_id, "chat".to_string(), args));
    })
}

fn truncate_markdown_context(content: String) -> String {
    let Some((end, _)) = content.char_indices().nth(MAX_CHAT_CONTEXT_CHARS) else {
        return content;
    };
    content[..end].to_string()
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
mod tests {
    use super::{MAX_CHAT_CONTEXT_CHARS, truncate_markdown_context};

    #[test]
    fn markdown_context_is_bounded_on_char_boundaries() {
        let content = "é".repeat(MAX_CHAT_CONTEXT_CHARS + 3);
        let bounded = truncate_markdown_context(content);
        assert_eq!(bounded.chars().count(), MAX_CHAT_CONTEXT_CHARS);
    }

    #[test]
    fn markdown_context_keeps_short_content() {
        let content = "# Note\n\nbody".to_string();
        assert_eq!(truncate_markdown_context(content.clone()), content);
    }
}
