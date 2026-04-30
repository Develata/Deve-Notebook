// apps/web/src/components/mobile_layout/chat_sheet.rs
//! plan_ref:
//!   - 08_ui_design_03_mobile#mobile-responsive-layout
//!   - 10_ai_agent#native-ai-chat-runtime
//!

use crate::components::chat::ChatPanel;
use crate::components::layout_context::ChatControl;
use crate::i18n::{Locale, t};
use leptos::prelude::*;

pub(crate) fn should_show_mobile_chat_sheet(
    visible: bool,
    drawer_open: bool,
    diff_open: bool,
    expanded: bool,
    keyboard_offset: i32,
) -> bool {
    visible && !drawer_open && !diff_open && (expanded || keyboard_offset <= 0)
}

pub(crate) fn mobile_chat_sheet_style(expanded: bool, keyboard_offset: i32) -> String {
    if expanded {
        if keyboard_offset > 0 {
            format!("bottom: {}px;", keyboard_offset)
        } else {
            String::new()
        }
    } else {
        let base = if keyboard_offset > 0 {
            keyboard_offset
        } else {
            58
        };
        format!("bottom: calc({}px + env(safe-area-inset-bottom));", base)
    }
}

#[component]
pub fn MobileChatSheet(
    keyboard_offset: ReadSignal<i32>,
    drawer_open: Signal<bool>,
    diff_open: Signal<bool>,
    expanded: ReadSignal<bool>,
    set_expanded: WriteSignal<bool>,
) -> impl IntoView {
    let locale = use_context::<RwSignal<Locale>>().expect("locale context");
    let chat_control = use_context::<ChatControl>();
    let visible = Signal::derive(move || {
        chat_control
            .as_ref()
            .map(|c| c.chat_visible.get())
            .unwrap_or(true)
    });
    let close_chat = Callback::new(move |_| set_expanded.set(false));

    view! {
        <Show when=move || {
            should_show_mobile_chat_sheet(
                visible.get(),
                drawer_open.get(),
                diff_open.get(),
                expanded.get(),
                keyboard_offset.get(),
            )
        }>
            <div
                class=move || if expanded.get() {
                    "fixed inset-0 z-[80] bg-panel transition-opacity duration-200 ease-out"
                } else {
                    "fixed right-2 z-[55]"
                }
                style=move || mobile_chat_sheet_style(expanded.get(), keyboard_offset.get())
                data-deve-mobile-chat=move || if expanded.get() { "expanded" } else { "collapsed" }
            >
                <Show
                    when=move || expanded.get()
                    fallback=move || {
                        view! {
                            <button
                                class="mobile-chat-chip h-11 min-w-[44px] px-3 rounded-full bg-panel border border-default shadow-sm text-sm font-medium text-primary active:bg-hover"
                                title=move || t::chat::toggle_mobile_chat(locale.get())
                                aria-label=move || t::chat::toggle_mobile_chat(locale.get())
                                on:click=move |_| set_expanded.set(true)
                            >
                                {move || t::chat::mobile_chip(locale.get())}
                            </button>
                        }
                    }
                >
                    <ChatPanel mobile=true on_close=close_chat />
                </Show>
            </div>
        </Show>
    }
}

#[cfg(test)]
mod tests {
    use super::{mobile_chat_sheet_style, should_show_mobile_chat_sheet};

    #[test]
    fn expanded_chat_stays_visible_when_keyboard_is_open() {
        assert!(should_show_mobile_chat_sheet(true, false, false, true, 280));
        assert_eq!(mobile_chat_sheet_style(true, 280), "bottom: 280px;");
    }

    #[test]
    fn collapsed_chip_hides_when_keyboard_is_open() {
        assert!(!should_show_mobile_chat_sheet(
            true, false, false, false, 280
        ));
    }

    #[test]
    fn drawer_and_diff_still_hide_mobile_chat() {
        assert!(!should_show_mobile_chat_sheet(true, true, false, true, 0));
        assert!(!should_show_mobile_chat_sheet(true, false, true, true, 0));
    }

    #[test]
    fn collapsed_chip_uses_footer_offset_when_keyboard_is_closed() {
        assert!(should_show_mobile_chat_sheet(true, false, false, false, 0));
        assert_eq!(
            mobile_chat_sheet_style(false, 0),
            "bottom: calc(58px + env(safe-area-inset-bottom));"
        );
    }
}
