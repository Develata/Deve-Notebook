// apps/web/src/components/chat/input_area.rs
//! plan_ref:
//!   - 10_ai_agent#native-ai-chat-runtime
//!
use crate::components::icons::*;
use crate::i18n::{Locale, t};
use leptos::prelude::*;

pub(crate) fn mobile_chat_input_area_class(mobile: bool) -> &'static str {
    if mobile {
        "p-2 border-t border-default bg-panel"
    } else {
        "p-3 border-t border-default bg-panel"
    }
}

pub(crate) fn mobile_chat_input_area_style(mobile: bool) -> &'static str {
    if mobile {
        "padding-bottom: calc(8px + env(safe-area-inset-bottom));"
    } else {
        ""
    }
}

pub(crate) fn mobile_chat_send_button_class(mobile: bool) -> &'static str {
    if mobile {
        "h-11 min-w-[44px] p-2 rounded active:bg-hover text-accent disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
    } else {
        "p-1.5 rounded hover:bg-hover text-accent disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
    }
}

pub(crate) fn mobile_chat_input_area_marker(mobile: bool) -> Option<&'static str> {
    mobile.then_some("keyboard-safe")
}

pub(crate) fn mobile_chat_input_marker(mobile: bool) -> Option<&'static str> {
    mobile.then_some("chat_input")
}

pub(crate) fn mobile_chat_send_button_marker(mobile: bool) -> Option<&'static str> {
    mobile.then_some("chat_send_button")
}

#[component]
pub fn InputArea(
    input: ReadSignal<String>,
    set_input: WriteSignal<String>,
    is_streaming: ReadSignal<bool>,
    send_message: Callback<()>,
    #[prop(optional)] mobile: bool,
) -> impl IntoView {
    let locale = use_context::<RwSignal<Locale>>().expect("locale context");
    view! {
        <div
            class=move || mobile_chat_input_area_class(mobile)
            style=move || mobile_chat_input_area_style(mobile)
            data-deve-mobile-chat-input-area=move || mobile_chat_input_area_marker(mobile)
        >
            <div class="relative rounded border border-default bg-input focus-within:border-b-accent transition-colors">
                <textarea
                    name="ai-chat-input"
                    data-deve-mobile-chat-input=move || mobile_chat_input_marker(mobile)
                    class="w-full max-h-32 p-2 bg-transparent border-none outline-none text-sm resize-none text-primary font-sans"
                    placeholder=move || t::chat::input_placeholder(locale.get())
                    rows="1"
                    prop:value=input
                    on:input=move |ev| set_input.set(event_target_value(&ev))
                    on:keydown={
                        let send_message = send_message.clone();
                        move |ev| {
                            if ev.key() == "Enter" && !ev.shift_key() {
                                ev.prevent_default();
                                send_message.run(());
                            }
                        }
                    }
                ></textarea>
                <div class="flex justify-between items-center px-2 pb-2">
                    <span class="text-[10px] text-muted">{move || t::chat::markdown_supported(locale.get())}</span>
                    <button
                        data-deve-mobile-chat-action=move || mobile_chat_send_button_marker(mobile)
                        data-deve-mobile-touch-target=move || mobile_chat_send_button_marker(mobile)
                        class=move || mobile_chat_send_button_class(mobile)
                        disabled=move || input.get().trim().is_empty() || is_streaming.get()
                        on:click=move |_| send_message.run(())
                        title=move || t::chat::send(locale.get())
                        aria-label=move || t::chat::send(locale.get())
                    >
                        <Send />
                    </button>
                </div>
            </div>
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::{
        mobile_chat_input_area_class, mobile_chat_input_area_marker, mobile_chat_input_area_style,
        mobile_chat_input_marker, mobile_chat_send_button_class, mobile_chat_send_button_marker,
    };

    #[test]
    fn mobile_chat_keyboard_input_area_uses_safe_area_padding() {
        assert!(mobile_chat_input_area_class(true).contains("p-2"));
        assert!(mobile_chat_input_area_style(true).contains("safe-area-inset-bottom"));
        assert_eq!(mobile_chat_input_area_style(false), "");
    }

    #[test]
    fn mobile_chat_keyboard_send_button_is_at_least_44px() {
        let class = mobile_chat_send_button_class(true);

        assert!(class.contains("h-11"));
        assert!(class.contains("min-w-[44px]"));
    }

    #[test]
    fn mobile_chat_keyboard_markers_are_mobile_only() {
        assert_eq!(mobile_chat_input_area_marker(true), Some("keyboard-safe"));
        assert_eq!(mobile_chat_input_marker(true), Some("chat_input"));
        assert_eq!(
            mobile_chat_send_button_marker(true),
            Some("chat_send_button")
        );
        assert_eq!(mobile_chat_input_area_marker(false), None);
        assert_eq!(mobile_chat_input_marker(false), None);
        assert_eq!(mobile_chat_send_button_marker(false), None);
    }
}
