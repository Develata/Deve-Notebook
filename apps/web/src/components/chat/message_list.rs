// apps/web/src/components/chat/message_list.rs
//! plan_ref:
//!   - 16_ai_agent#native-ai-chat-runtime
//!
use crate::components::chat::empty_state::EmptyState;
use crate::components::chat::message_item::{MessageItem, build_message_index};
use crate::components::chat::slash_commands::ChatSessionMode;
use crate::i18n::{Locale, t};
use crate::runtime::domain::ChatMessage;
use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use leptos::html;
use leptos::prelude::*;
use std::collections::HashMap;
use wasm_bindgen::JsCast;

pub(crate) fn should_consume_apply_click(session_mode: ChatSessionMode) -> bool {
    session_mode == ChatSessionMode::Build
}

#[component]
pub fn MessageList(
    messages: ReadSignal<Vec<ChatMessage>>,
    is_streaming: ReadSignal<bool>,
    session_mode: ReadSignal<ChatSessionMode>,
    send_example: Callback<String>,
    on_apply: Callback<String>,
    #[prop(optional)] mobile: bool,
) -> impl IntoView {
    let locale = use_context::<RwSignal<Locale>>().expect("locale context");
    let messages_end_ref = NodeRef::<html::Div>::new();
    let message_indices: Memo<HashMap<u64, usize>> =
        Memo::new(move |_| messages.with(|items| build_message_index(items)));

    Effect::new(move |_| {
        messages.track();
        if let Some(el) = messages_end_ref.get() {
            el.scroll_into_view();
        }
    });

    let on_click = move |ev: web_sys::MouseEvent| {
        if !should_consume_apply_click(session_mode.get_untracked()) {
            return;
        }
        let mut el = ev
            .target()
            .and_then(|t| t.dyn_into::<web_sys::Element>().ok());
        while let Some(node) = el {
            if node.class_list().contains("apply-code")
                && let Some(data) = node.get_attribute("data-code")
                && let Ok(bytes) = STANDARD.decode(data)
                && let Ok(code) = String::from_utf8(bytes)
            {
                on_apply.run(code);
                break;
            }
            el = node.parent_element();
        }
    };

    view! {
        <div class=move || if mobile {
            "flex-1 overflow-y-auto p-2.5 space-y-3"
        } else {
            "flex-1 overflow-y-auto p-4 space-y-4"
        } on:click=on_click>
            {move || if messages.get().is_empty() {
                view! { <EmptyState send_example=send_example.clone() /> }.into_any()
            } else {
                view! {
                    <For
                        each=move || messages.with(|items| {
                            items.iter().map(|message| message.ui_id).collect::<Vec<_>>()
                        })
                        key=|message_id| *message_id
                        children=move |message_id| view! {
                            <MessageItem
                                messages=messages
                                message_indices=message_indices
                                message_id=message_id
                                session_mode=session_mode
                                mobile=mobile
                            />
                        }
                    />
                }.into_any()
            }}

            {move || if is_streaming.get() {
                view! {
                    <div class="flex items-center gap-2 text-xs text-secondary px-1">
                        <span class="animate-pulse">{move || t::chat::thinking(locale.get())}</span>
                    </div>
                }.into_any()
            } else {
                view! {}.into_any()
            }}

            <div node_ref=messages_end_ref></div>
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::should_consume_apply_click;
    use crate::components::chat::slash_commands::ChatSessionMode;

    #[test]
    fn chat_apply_click_is_consumed_only_in_build_mode() {
        assert!(!should_consume_apply_click(ChatSessionMode::Plan));
        assert!(should_consume_apply_click(ChatSessionMode::Build));
    }

    #[test]
    fn chat_rows_use_ui_identity_and_skip_global_code_highlight() {
        let source = include_str!("message_list.rs");
        let ui_field = ["message", ".", "ui_id"].concat();
        let identity_key = ["key=", "|message_id|", " *", "message_id"].concat();
        let legacy_index_prop = ["message_", "index=message_", "index"].concat();
        let legacy_key = [
            "msg",
            ".",
            "req_id",
            ".",
            "clone",
            "()",
            ".",
            "unwrap_or_else",
        ]
        .concat();
        let global_highlight = ["highlight", "All"].concat();

        assert!(source.contains(&ui_field));
        assert!(source.contains(&identity_key));
        assert!(source.contains("Memo<HashMap<u64, usize>>"));
        assert!(source.contains("message_indices=message_indices"));
        assert!(!source.contains(&legacy_index_prop));
        assert!(!source.contains(&legacy_key));
        assert!(!source.contains(&global_highlight));
    }
}
