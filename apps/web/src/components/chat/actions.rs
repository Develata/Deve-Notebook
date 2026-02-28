// apps/web/src/components/chat/actions.rs
use crate::editor::ffi::getEditorContent;
use crate::hooks::use_core::CoreState;
use deve_core::models::Op;
use deve_core::protocol::ClientMessage;
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

        let context = serde_json::json!({ "current_file": current_doc_path });
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

pub fn make_on_apply(core: CoreState) -> Callback<String> {
    Callback::new(move |code: String| {
        let Some(doc_id) = core.current_doc.get_untracked() else {
            leptos::logging::warn!("No active doc to apply code.");
            return;
        };
        let utf16_len = getEditorContent().encode_utf16().count();
        let pos = match u32::try_from(utf16_len) {
            Ok(v) => v,
            Err(_) => {
                leptos::logging::warn!("Apply code aborted: UTF-16 length overflow.");
                return;
            }
        };
        let op = Op::Insert {
            pos,
            content: code.into(),
        };
        let client_id = (js_sys::Math::random() * 1_000_000.0) as u64;
        core.ws.send(ClientMessage::Edit {
            doc_id,
            op,
            client_id,
        });
    })
}
