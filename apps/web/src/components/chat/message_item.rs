// apps/web/src/components/chat/message_item.rs
//! plan_ref:
//!   - 16_ai_agent#native-ai-chat-runtime
//!   - 10_rendering#markdown-render-whitelist
//!
use crate::components::chat::slash_commands::ChatSessionMode;
use crate::i18n::{Locale, t};
use crate::runtime::domain::ChatMessage;
use crate::utils::{markdown::render_markdown, time::format_time_of_day};
use leptos::html;
use leptos::prelude::*;
use std::collections::HashMap;
use wasm_bindgen::{JsCast, JsValue};

fn render_chat_math(element: &web_sys::HtmlElement) -> bool {
    let Some(window) = web_sys::window() else {
        return false;
    };
    let Ok(bridge) = js_sys::Reflect::get(window.as_ref(), &JsValue::from_str("__deveWebBridge"))
    else {
        return false;
    };
    let Ok(call) = js_sys::Reflect::get(&bridge, &JsValue::from_str("call")) else {
        return false;
    };
    let Some(function) = call.dyn_ref::<js_sys::Function>() else {
        return false;
    };
    function
        .call2(
            &bridge,
            &JsValue::from_str("renderChatMath"),
            element.as_ref(),
        )
        .ok()
        .and_then(|value| value.as_bool())
        .unwrap_or(false)
}

fn highlight_chat_code_blocks(element: &web_sys::HtmlElement) {
    let Some(window) = web_sys::window() else {
        return;
    };
    let Ok(hljs) = js_sys::Reflect::get(window.as_ref(), &JsValue::from_str("hljs")) else {
        return;
    };
    let Ok(highlight) = js_sys::Reflect::get(&hljs, &JsValue::from_str("highlightElement")) else {
        return;
    };
    let Some(highlight) = highlight.dyn_ref::<js_sys::Function>() else {
        return;
    };
    let Ok(nodes) = element.query_selector_all("pre code") else {
        return;
    };
    for index in 0..nodes.length() {
        let Some(node) = nodes.item(index) else {
            continue;
        };
        let Ok(code) = node.dyn_into::<web_sys::HtmlElement>() else {
            continue;
        };
        let _ = highlight.call1(&hljs, code.as_ref());
    }
}

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

pub(crate) fn build_message_index(messages: &[ChatMessage]) -> HashMap<u64, usize> {
    messages
        .iter()
        .enumerate()
        .map(|(index, message)| (message.ui_id, index))
        .collect()
}

fn message_for_id<'a>(
    messages: &'a [ChatMessage],
    message_indices: &HashMap<u64, usize>,
    message_id: u64,
) -> Option<&'a ChatMessage> {
    message_indices
        .get(&message_id)
        .and_then(|index| messages.get(*index))
        .filter(|message| message.ui_id == message_id)
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
    messages: ReadSignal<Vec<ChatMessage>>,
    message_indices: Memo<HashMap<u64, usize>>,
    message_id: u64,
    session_mode: ReadSignal<ChatSessionMode>,
    #[prop(optional)] mobile: bool,
) -> impl IntoView {
    let locale = use_context::<RwSignal<Locale>>().expect("locale context");
    let message_metadata = Memo::new(move |_| {
        message_indices.with(|indices| {
            messages.with(|items| {
                message_for_id(items, indices, message_id)
                    .map(|message| (message.role == "user", message.ts_ms))
                    .unwrap_or((false, 0))
            })
        })
    });
    let is_user = Memo::new(move |_| message_metadata.get().0);
    let ts_ms = Memo::new(move |_| message_metadata.get().1);
    let content_revision = Memo::new(move |_| {
        message_indices.with(|indices| {
            messages.with(|items| {
                message_for_id(items, indices, message_id)
                    .map(|message| message.content_revision)
                    .unwrap_or(0)
            })
        })
    });
    let body_ref = NodeRef::<html::Div>::new();
    let sender_text = move || {
        if is_user.get() {
            t::chat::you(locale.get())
        } else {
            t::chat::assistant(locale.get())
        }
    };
    let content_html = move || {
        let _ = content_revision.get();
        let apply_label = should_show_apply_label(is_user.get(), session_mode.get())
            .then(|| t::chat::apply(locale.get()));
        message_indices.with(|indices| {
            messages.with_untracked(|items| {
                let content = message_for_id(items, indices, message_id)
                    .map(|message| message.content.as_str())
                    .unwrap_or_default();
                render_markdown(content, apply_label)
            })
        })
    };
    let ts_text = move || format_time_of_day(ts_ms.get(), locale.get());
    Effect::new(move |_| {
        let _ = content_revision.get();
        session_mode.track();
        locale.track();
        if let Some(el) = body_ref.get() {
            let _ = render_chat_math(&el);
            highlight_chat_code_blocks(&el);
        }
    });

    view! {
        <div class="flex flex-col gap-1">
            <div class=move || format!("flex items-center gap-2 {}", if is_user.get() { "flex-row-reverse" } else { "flex-row" })>
                <div class=move || format!("w-6 h-6 rounded flex items-center justify-center text-xs font-bold {}",
                    if is_user.get() { "bg-accent text-on-accent" } else { "bg-panel text-primary" }
                )>
                    {move || if is_user.get() { "U" } else { "AI" }}
                </div>
                <span class="text-xs text-muted">{sender_text}</span>
            </div>

            <div
                class=move || chat_message_bubble_class(is_user.get(), mobile)
                data-deve-mobile-chat-message=move || mobile_chat_message_marker(mobile)
            >
                <div
                    node_ref=body_ref
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
        build_message_index, chat_markdown_body_class, chat_message_bubble_class, message_for_id,
        mobile_chat_code_scroll_marker, mobile_chat_message_marker, mobile_chat_timestamp_marker,
        mobile_chat_wrap_marker, should_show_apply_label,
    };
    use crate::components::chat::slash_commands::ChatSessionMode;
    use crate::runtime::domain::ChatMessage;

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

    #[test]
    fn chat_row_lookup_follows_prepend_and_reorder() {
        let first = ChatMessage::new("assistant", "first", None, 1);
        let second = ChatMessage::new("user", "second", None, 2);
        let prepended = ChatMessage::new("assistant", "prepended", None, 3);
        let messages = vec![prepended, second.clone(), first.clone()];
        let message_indices = build_message_index(&messages);

        assert_eq!(message_indices.get(&first.ui_id), Some(&2));
        assert_eq!(message_indices.get(&second.ui_id), Some(&1));
        assert_eq!(
            message_for_id(&messages, &message_indices, first.ui_id)
                .map(|message| message.content.as_str()),
            Some("first")
        );
        assert_eq!(
            message_for_id(&messages, &message_indices, second.ui_id)
                .map(|message| message.content.as_str()),
            Some("second")
        );
    }
}
