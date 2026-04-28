// apps/web/src/components/chat/panel.rs
//! plan_ref:
//!   - 10_ai_agent#native-ai-chat-runtime
//!   - 08_ui_design_01_web#web-layout-persistence
//!
use crate::components::chat::actions::{
    make_on_apply, make_send_example, make_send_message, make_send_text,
};
use crate::components::chat::drag_overlay::DragOverlay;
use crate::components::chat::drop_handler::{on_drag_leave, on_drag_over, on_drop};
use crate::components::chat::header::ChatHeader;
use crate::components::chat::input_area::InputArea;
use crate::components::chat::message_list::MessageList;
use crate::components::chat::slash_commands::ChatSessionMode;
use crate::hooks::use_core::CoreState;
use crate::i18n::Locale;
use deve_core::protocol::ServerErrorCode;
use leptos::prelude::*;

#[path = "panel_effects.rs"]
mod panel_effects;
#[path = "panel_status.rs"]
mod panel_status;

use self::panel_effects::{attach_plugin_response_effect, attach_scope_reset_effect};
use self::panel_status::{error_notice, loading_notice};

#[component]
pub fn ChatPanel(#[prop(optional)] mobile: bool, on_close: Callback<()>) -> impl IntoView {
    let core = expect_context::<CoreState>();
    let locale = use_context::<RwSignal<Locale>>().expect("locale context");
    let (input, set_input) = signal(String::new());
    let (is_drag_over, set_is_drag_over) = signal(false);
    let (last_prompt, set_last_prompt) = signal(String::new());
    let (error_code, set_error_code) = signal(None::<ServerErrorCode>);
    let (pending_reqs, set_pending_reqs) = signal(Vec::<String>::new());
    let (session_mode, set_session_mode) = signal(ChatSessionMode::Plan);

    let messages = core.chat_messages;
    let is_streaming = core.is_chat_streaming;

    let on_req_id = Callback::new(move |req_id: String| {
        set_error_code.set(None);
        set_pending_reqs.update(|v| v.push(req_id));
    });
    let on_user_text = Callback::new(move |msg: String| {
        set_last_prompt.set(msg);
    });
    let mode_core = core.clone();
    let on_mode_change = Callback::new(move |mode: ChatSessionMode| {
        let content = match mode {
            ChatSessionMode::Plan => crate::i18n::t::chat::switched_to_plan(locale.get_untracked()),
            ChatSessionMode::Build => {
                crate::i18n::t::chat::switched_to_build(locale.get_untracked())
            }
        };
        mode_core.append_chat_message("assistant", content, None);
    });

    let send_text = make_send_text(
        core.clone(),
        is_streaming,
        session_mode,
        set_session_mode,
        Some(on_req_id),
        Some(on_user_text),
        Some(on_mode_change),
    );
    let send_message = make_send_message(input, set_input, is_streaming, send_text.clone());
    let send_example = make_send_example(send_text.clone(), set_input);
    let on_apply = make_on_apply(core.clone());
    let retry = Callback::new(move |_| {
        let prompt = last_prompt.get_untracked();
        if !prompt.is_empty() {
            send_text.run(prompt);
        }
    });

    attach_scope_reset_effect(
        core.clone(),
        set_pending_reqs,
        set_error_code,
        set_last_prompt,
    );
    attach_plugin_response_effect(core.clone(), pending_reqs, set_pending_reqs, set_error_code);

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
            on:drop=on_drop(set_input, set_is_drag_over, core.set_sync_banner)
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
