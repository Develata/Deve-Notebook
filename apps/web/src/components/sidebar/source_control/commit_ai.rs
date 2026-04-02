use crate::hooks::use_core::{ChatContext, ChatMessage, SourceControlContext};
use crate::i18n::{Locale, t};
use leptos::prelude::*;

pub fn build_generate_callback(
    core: SourceControlContext,
    chat_ctx: ChatContext,
    locale: RwSignal<Locale>,
    active_req_id: RwSignal<Option<String>>,
    saw_streaming: RwSignal<bool>,
    set_is_generating: WriteSignal<bool>,
) -> Callback<()> {
    Callback::new(move |_| {
        if !core.can_write.get_untracked() || core.staged_changes.get_untracked().is_empty() {
            return;
        }
        let req_id = uuid::Uuid::new_v4().to_string();
        let joined_paths = core
            .staged_changes
            .get()
            .into_iter()
            .map(|entry| entry.path)
            .collect::<Vec<_>>()
            .join("\n");
        let prompt = format!(
            "{}\n{}",
            t::source_control::generate_prompt(locale.get()),
            joined_paths
        );
        let args = vec![
            serde_json::json!(req_id),
            serde_json::json!(prompt),
            serde_json::json!(""),
        ];
        active_req_id.set(Some(req_id.clone()));
        saw_streaming.set(false);
        set_is_generating.set(true);
        chat_ctx.set_messages.update(|messages| {
            messages.push(ChatMessage {
                role: "assistant".into(),
                content: String::new(),
                req_id: Some(req_id.clone()),
                ts_ms: js_sys::Date::now() as u64,
            });
        });
        chat_ctx.set_is_streaming.set(true);
        chat_ctx
            .on_plugin_call
            .run((req_id, "agent-bridge".to_string(), "chat".to_string(), args));
    })
}

pub fn sync_generated_commit_message(
    chat_ctx: ChatContext,
    active_req_id: RwSignal<Option<String>>,
    saw_streaming: RwSignal<bool>,
    set_msg: WriteSignal<String>,
    set_is_generating: WriteSignal<bool>,
) {
    Effect::new(move |_| {
        let req_id = active_req_id.get();
        let is_streaming = chat_ctx.is_streaming.get();
        if let Some(req_id) = req_id {
            if let Some(content) = chat_ctx
                .messages
                .get()
                .iter()
                .rev()
                .find(|m| m.req_id.as_deref() == Some(req_id.as_str()))
                .map(|m| m.content.clone())
            {
                set_msg.set(content);
            }
            if is_streaming {
                saw_streaming.set(true);
            }
            if saw_streaming.get_untracked() && !is_streaming {
                set_is_generating.set(false);
                saw_streaming.set(false);
                active_req_id.set(None);
            }
        }
    });
}
