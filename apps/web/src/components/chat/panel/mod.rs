// apps/web/src/components/chat/panel.rs
//! plan_ref:
//!   - 16_ai_agent#native-ai-chat-runtime
//!   - 11_ui_design/01_web#web-layout-persistence
//!
use crate::components::chat::actions::{
    ChatSendControls, make_on_apply, make_send_example, make_send_message, make_send_text,
};
use crate::components::chat::drag_overlay::DragOverlay;
use crate::components::chat::drop_handler::{on_drag_leave, on_drag_over, on_drop};
use crate::components::chat::header::ChatHeader;
use crate::components::chat::input_area::InputArea;
use crate::components::chat::message_list::MessageList;
use crate::components::chat::slash_commands::ChatSessionMode;
use crate::hooks::use_core::{ChatContext, EditorContext};
use crate::i18n::Locale;
use crate::runtime::domain::ChatMessage;
use crate::runtime::{
    document_client::DocumentClient, scope_client::ScopeClient, session_client::SessionClient,
};
use deve_core::protocol::ServerErrorCode;
use leptos::prelude::*;

mod effects;
mod status;

use self::effects::{attach_plugin_response_effect, attach_scope_reset_effect};
use self::status::{error_notice, loading_notice};

pub(crate) fn chat_retry_prompt(last_prompt: &str) -> Option<String> {
    let prompt = last_prompt.trim();
    (!prompt.is_empty()).then(|| prompt.to_string())
}

#[component]
pub fn ChatPanel(#[prop(optional)] mobile: bool, on_close: Callback<()>) -> impl IntoView {
    let chat = expect_context::<ChatContext>();
    let document = expect_context::<DocumentClient>();
    let editor = expect_context::<EditorContext>();
    let scope = expect_context::<ScopeClient>();
    let session = expect_context::<SessionClient>();
    let locale = use_context::<RwSignal<Locale>>().expect("locale context");
    let (input, set_input) = signal(String::new());
    let (is_drag_over, set_is_drag_over) = signal(false);
    let (last_prompt, set_last_prompt) = signal(String::new());
    let (error_code, set_error_code) = signal(None::<ServerErrorCode>);
    let (pending_reqs, set_pending_reqs) = signal(Vec::<String>::new());
    let (session_mode, set_session_mode) = signal(ChatSessionMode::Plan);

    let messages = chat.messages;
    let is_streaming = chat.is_streaming;

    let on_req_id = Callback::new(move |req_id: String| {
        set_error_code.set(None);
        set_pending_reqs.update(|v| v.push(req_id));
    });
    let on_user_text = Callback::new(move |msg: String| {
        set_last_prompt.set(msg);
    });
    let mode_chat = chat.clone();
    let on_mode_change = Callback::new(move |mode: ChatSessionMode| {
        let content = match mode {
            ChatSessionMode::Plan => crate::i18n::t::chat::switched_to_plan(locale.get_untracked()),
            ChatSessionMode::Build => {
                crate::i18n::t::chat::switched_to_build(locale.get_untracked())
            }
        };
        append_chat_message(&mode_chat, "assistant", content.to_string(), None);
    });

    let send_text = make_send_text(
        crate::components::chat::actions::ChatSendRuntime {
            chat: chat.clone(),
            document: document.clone(),
        },
        ChatSendControls {
            is_streaming,
            locale,
            session_mode,
            set_session_mode,
            on_req_id: Some(on_req_id),
            on_user_text: Some(on_user_text),
            on_mode_change: Some(on_mode_change),
        },
    );
    let send_message = make_send_message(input, set_input, is_streaming, send_text.clone());
    let send_example = make_send_example(send_text.clone(), set_input);
    let on_apply = make_on_apply(crate::components::chat::actions::ChatApplyRuntime {
        session: session.clone(),
        editor: editor.clone(),
        locale,
    });
    let retry = Callback::new(move |_| {
        if let Some(prompt) = chat_retry_prompt(&last_prompt.get_untracked()) {
            send_text.run(prompt);
        }
    });

    attach_scope_reset_effect(
        scope,
        editor,
        set_pending_reqs,
        set_error_code,
        set_last_prompt,
    );
    attach_plugin_response_effect(chat.clone(), pending_reqs, set_pending_reqs, set_error_code);

    let loading = Signal::derive(move || is_streaming.get() || !pending_reqs.get().is_empty());

    view! {
        <div
            class=move || if mobile {
                "h-full flex flex-col bg-sidebar relative"
            } else {
                "h-full flex flex-col bg-sidebar border-l border-default relative"
            }
            on:dragover=on_drag_over(set_is_drag_over)
            on:dragleave=on_drag_leave(set_is_drag_over)
            on:drop=on_drop(set_input, set_is_drag_over, session.set_sync_banner, locale)
        >
            <DragOverlay is_drag_over=is_drag_over />
            <ChatHeader mobile=mobile on_close=on_close session_mode=session_mode />
            <MessageList
                messages=messages
                is_streaming=is_streaming
                session_mode=session_mode
                send_example=send_example
                on_apply=on_apply
                mobile=mobile
            />
            {error_notice(error_code, locale, retry)}
            {loading_notice(loading, locale)}
            <InputArea
                input=input
                set_input=set_input
                is_streaming=is_streaming
                send_message=send_message
                mobile=mobile
            />
        </div>
    }
}

fn append_chat_message(chat: &ChatContext, role: &str, content: String, req_id: Option<String>) {
    chat.set_messages.update(|msgs| {
        msgs.push(ChatMessage {
            role: role.to_string(),
            content,
            req_id,
            ts_ms: js_sys::Date::now() as u64,
        });
    });
}

#[cfg(test)]
mod tests {
    use super::chat_retry_prompt;

    #[test]
    fn mobile_chat_error_retry_uses_last_prompt() {
        assert_eq!(
            chat_retry_prompt("trigger_error"),
            Some("trigger_error".to_string())
        );
    }

    #[test]
    fn mobile_chat_error_retry_ignores_empty_prompt() {
        assert_eq!(chat_retry_prompt(""), None);
        assert_eq!(chat_retry_prompt("   "), None);
    }
}
