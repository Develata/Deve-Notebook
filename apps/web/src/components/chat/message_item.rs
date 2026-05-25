// apps/web/src/components/chat/message_item.rs
//! plan_ref:
//!   - 16_ai_agent#native-ai-chat-runtime
//!
use crate::components::chat::slash_commands::ChatSessionMode;
use crate::hooks::use_core::types::ChatMessage;
use crate::i18n::{Locale, t};
use crate::utils::{markdown::render_markdown, time::format_time_of_day};
use leptos::prelude::*;
use wasm_bindgen::JsCast;

pub(crate) fn should_show_apply_label(
    is_user_message: bool,
    session_mode: ChatSessionMode,
) -> bool {
    !is_user_message && session_mode == ChatSessionMode::Build
}

pub(crate) fn chat_message_bubble_class(is_user: bool, mobile: bool) -> String {
    format!(
        "rounded px-3 py-2 text-sm leading-relaxed {} {}",
        if mobile { "max-w-[96%]" } else { "max-w-[90%]" },
        if is_user {
            if mobile {
                "bg-chat-user text-primary self-end ml-3"
            } else {
                "bg-chat-user text-primary self-end ml-8"
            }
        } else if mobile {
            "bg-panel text-primary border border-default self-start mr-3"
        } else {
            "bg-panel text-primary border border-default self-start mr-8"
        }
    )
}

pub(crate) fn chat_markdown_body_class(_mobile: bool) -> &'static str {
    "markdown-body break-words overflow-x-auto"
}

pub(crate) fn mobile_chat_message_marker(mobile: bool) -> Option<&'static str> {
    mobile.then_some("readable")
}

pub(crate) fn mobile_chat_wrap_marker(mobile: bool) -> Option<&'static str> {
    mobile.then_some("break-words")
}

pub(crate) fn mobile_chat_code_scroll_marker(mobile: bool) -> Option<&'static str> {
    mobile.then_some("horizontal")
}

pub(crate) fn mobile_chat_timestamp_marker(mobile: bool) -> Option<&'static str> {
    mobile.then_some("visible")
}

/// Handles click events on markdown content.
/// Prevents link navigation unless Ctrl/Meta key is pressed.
fn handle_link_click(ev: web_sys::MouseEvent) {
    // Check if click target is an <a> element
    let Some(target) = ev.target() else { return };
    let Ok(el) = target.dyn_into::<web_sys::HtmlElement>() else {
        return;
    };

    // Use closest() to handle clicks on nested elements within <a>
    if el.closest("a").ok().flatten().is_some() {
        // Only allow navigation when Ctrl/Meta is pressed
        if !ev.ctrl_key() && !ev.meta_key() {
            ev.prevent_default();
        }
    }
}

#[component]
pub fn MessageItem(
    msg: ChatMessage,
    session_mode: ReadSignal<ChatSessionMode>,
    #[prop(optional)] mobile: bool,
) -> impl IntoView {
    let locale = use_context::<RwSignal<Locale>>().expect("locale context");
    let is_user = msg.role == "user";
    let content = msg.content.clone();
    let sender_text = move || {
        if is_user {
            t::chat::you(locale.get())
        } else {
            t::chat::assistant(locale.get())
        }
    };
    let content_html = move || {
        let apply_label = should_show_apply_label(is_user, session_mode.get())
            .then(|| t::chat::apply(locale.get()));
        render_markdown(&content, apply_label)
    };
    let ts_ms = msg.ts_ms;
    let ts_text = move || format_time_of_day(ts_ms, locale.get());

    view! {
        <div class="flex flex-col gap-1">
            <div class={format!("flex items-center gap-2 {}", if is_user { "flex-row-reverse" } else { "flex-row" })}>
                <div class={format!("w-6 h-6 rounded flex items-center justify-center text-xs font-bold {}",
                    if is_user { "bg-accent text-on-accent" } else { "bg-panel text-primary" }
                )}>
                    {if is_user { "U" } else { "AI" }}
                </div>
                <span class="text-xs text-muted">{sender_text}</span>
            </div>

            <div
                class=chat_message_bubble_class(is_user, mobile)
                data-deve-mobile-chat-message=move || mobile_chat_message_marker(mobile)
            >
                <div
                    class=move || chat_markdown_body_class(mobile)
                    data-deve-mobile-chat-wrap=move || mobile_chat_wrap_marker(mobile)
                    data-deve-mobile-chat-code-scroll=move || mobile_chat_code_scroll_marker(mobile)
                    inner_html=content_html
                    on:click=handle_link_click
                ></div>
                <div
                    class="mt-1 text-[10px] text-muted text-right"
                    data-deve-mobile-chat-timestamp=move || mobile_chat_timestamp_marker(mobile)
                >
                    {ts_text}
                </div>
            </div>
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::{
        chat_markdown_body_class, chat_message_bubble_class, mobile_chat_code_scroll_marker,
        mobile_chat_message_marker, mobile_chat_timestamp_marker, mobile_chat_wrap_marker,
        should_show_apply_label,
    };
    use crate::components::chat::slash_commands::ChatSessionMode;

    #[test]
    fn chat_apply_label_is_build_only_for_assistant_messages() {
        assert!(!should_show_apply_label(false, ChatSessionMode::Plan));
        assert!(should_show_apply_label(false, ChatSessionMode::Build));
        assert!(!should_show_apply_label(true, ChatSessionMode::Build));
    }

    #[test]
    fn mobile_chat_readability_bubble_keeps_wide_wrap_surface() {
        let assistant = chat_message_bubble_class(false, true);
        let user = chat_message_bubble_class(true, true);

        assert!(assistant.contains("max-w-[96%]"));
        assert!(assistant.contains("self-start"));
        assert!(user.contains("max-w-[96%]"));
        assert!(user.contains("self-end"));
    }

    #[test]
    fn mobile_chat_readability_markdown_wrap_and_code_scroll_are_bound() {
        let class = chat_markdown_body_class(true);

        assert!(class.contains("break-words"));
        assert!(class.contains("overflow-x-auto"));
        assert_eq!(mobile_chat_wrap_marker(true), Some("break-words"));
        assert_eq!(mobile_chat_code_scroll_marker(true), Some("horizontal"));
    }

    #[test]
    fn mobile_chat_readability_markers_are_mobile_only() {
        assert_eq!(mobile_chat_message_marker(true), Some("readable"));
        assert_eq!(mobile_chat_timestamp_marker(true), Some("visible"));
        assert_eq!(mobile_chat_message_marker(false), None);
        assert_eq!(mobile_chat_wrap_marker(false), None);
        assert_eq!(mobile_chat_code_scroll_marker(false), None);
        assert_eq!(mobile_chat_timestamp_marker(false), None);
    }
}
