use crate::editor::ffi::try_get_editor_selection;
use crate::hooks::use_core::CoreState;
use leptos::prelude::*;

pub fn make_send_text(
    core: CoreState,
    is_streaming: ReadSignal<bool>,
    on_req_id: Option<Callback<String>>,
    on_user_text: Option<Callback<String>>,
) -> Callback<String> {
    Callback::new(move |msg: String| {
        let msg = msg.trim().to_string();
        if msg.is_empty() || is_streaming.get() {
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
        let sel_json = try_get_editor_selection().unwrap_or_else(|| "null".to_string());
        let selection = serde_json::from_str(&sel_json).unwrap_or(serde_json::Value::Null);
        let context = serde_json::json!({
            "current_file": current_doc_path,
            "selection": selection,
        });
        let args = vec![serde_json::json!(req_id), serde_json::json!(msg), context];
        let plugin_id = core.ai_mode.get_untracked();
        core.on_plugin_call
            .run((req_id, plugin_id, "chat".to_string(), args));
    })
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
