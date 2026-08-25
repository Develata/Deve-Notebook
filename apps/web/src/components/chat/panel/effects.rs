//! plan_ref:
//!   - 16_ai_agent#native-ai-chat-runtime
//!   - 07_network#web-ws-runtime
//!
use crate::hooks::use_core::{ChatContext, EditorContext};
use crate::runtime::domain::ChatMessage;
use crate::runtime::scope_client::ScopeClient;
use deve_core::protocol::ServerErrorCode;
use leptos::prelude::*;

pub fn attach_scope_reset_effect(
    scope: ScopeClient,
    editor: EditorContext,
    set_pending_reqs: WriteSignal<Vec<String>>,
    set_error_code: WriteSignal<Option<ServerErrorCode>>,
    set_last_prompt: WriteSignal<String>,
) {
    Effect::new(move |_| {
        let _ = (
            scope.current_repo.get(),
            editor.active_branch.get(),
            editor.pending_repo_switch.get(),
            editor.pending_branch_switch.get(),
        );
        set_pending_reqs.set(Vec::new());
        set_error_code.set(None);
        set_last_prompt.set(String::new());
    });
}

pub fn attach_plugin_response_effect(
    chat: ChatContext,
    pending_reqs: ReadSignal<Vec<String>>,
    set_pending_reqs: WriteSignal<Vec<String>>,
    set_error_code: WriteSignal<Option<ServerErrorCode>>,
) {
    let (last_handled_req, set_last_handled_req) = signal(None::<String>);
    Effect::new(move |_| {
        let Some((req_id, _result, error)) = chat.plugin_last_response.get() else {
            return;
        };
        if last_handled_req.get_untracked().as_deref() == Some(req_id.as_str()) {
            return;
        }
        let matched = chat_response_matches_panel(
            &req_id,
            &pending_reqs.get_untracked(),
            &chat.messages.get(),
        );
        if !matched {
            return;
        }
        set_pending_reqs.update(|v| v.retain(|id| id != &req_id));
        set_last_handled_req.set(Some(req_id.clone()));
        if let Some(err) = error {
            if let Some(detail) = err.detail.as_deref() {
                leptos::logging::warn!("Plugin request {} failed: {}", req_id, detail);
            }
            set_error_code.set(Some(err.code));
        }
    });
}

fn chat_response_matches_panel(
    req_id: &str,
    pending_reqs: &[String],
    messages: &[ChatMessage],
) -> bool {
    pending_reqs.iter().any(|id| id == req_id)
        || messages
            .iter()
            .any(|message| message.req_id.as_deref() == Some(req_id))
}

#[cfg(test)]
mod tests {
    use super::chat_response_matches_panel;
    use crate::runtime::domain::ChatMessage;

    fn chat_message(req_id: Option<&str>) -> ChatMessage {
        ChatMessage::new("assistant", String::new(), req_id.map(str::to_string), 0)
    }

    #[test]
    fn plugin_response_matches_pending_request() {
        assert!(chat_response_matches_panel(
            "req-1",
            &[String::from("req-1")],
            &[]
        ));
    }

    #[test]
    fn plugin_response_matches_chat_placeholder_when_pending_was_missed() {
        assert!(chat_response_matches_panel(
            "req-1",
            &[],
            &[chat_message(Some("req-1"))]
        ));
    }

    #[test]
    fn unrelated_plugin_response_is_ignored() {
        assert!(!chat_response_matches_panel(
            "req-1",
            &[],
            &[chat_message(Some("req-2")), chat_message(None)]
        ));
    }
}
