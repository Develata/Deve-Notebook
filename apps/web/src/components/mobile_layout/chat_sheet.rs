// apps/web/src/components/mobile_layout/chat_sheet.rs
//! plan_ref:
//!   - 11_ui_design/03_mobile#mobile-responsive-layout
//!   - 16_ai_agent#native-ai-chat-runtime
//!

use crate::components::chat::ChatPanel;
use crate::components::layout_context::ChatControl;
use crate::i18n::{Locale, t};
use leptos::prelude::*;

pub(crate) fn should_show_mobile_chat_sheet(
    visible: bool,
    drawer_open: bool,
    diff_open: bool,
    surface_switcher_sheet_visible: bool,
    expanded: bool,
    keyboard_offset: i32,
) -> bool {
    visible
        && !drawer_open
        && !diff_open
        && !surface_switcher_sheet_visible
        && (expanded || keyboard_offset <= 0)
}

pub(crate) fn mobile_chat_runtime_conflict_should_close(
    visible: bool,
    drawer_open: bool,
    diff_open: bool,
    surface_switcher_sheet_visible: bool,
    expanded: bool,
) -> bool {
    expanded && (!visible || drawer_open || diff_open || surface_switcher_sheet_visible)
}

pub(crate) fn mobile_chat_sheet_style(expanded: bool, keyboard_offset: i32) -> String {
    if expanded {
        if keyboard_offset > 0 {
            format!(
                "padding-top: env(safe-area-inset-top); bottom: {}px;",
                keyboard_offset
            )
        } else {
            "padding-top: env(safe-area-inset-top);".to_string()
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

pub(crate) fn mobile_chat_sheet_class(expanded: bool) -> &'static str {
    if expanded {
        "fixed inset-0 z-[var(--z-overlay)] bg-panel transition-opacity duration-200 ease-out"
    } else {
        "fixed right-2 z-[var(--z-floating)]"
    }
}

pub(crate) fn mobile_chat_page_mode(expanded: bool) -> &'static str {
    if expanded { "fullscreen" } else { "chip" }
}

pub(crate) fn mobile_chat_after_open() -> bool {
    true
}

pub(crate) fn mobile_chat_after_close() -> bool {
    false
}

#[component]
pub fn MobileChatSheet(
    keyboard_offset: ReadSignal<i32>,
    drawer_open: Signal<bool>,
    diff_open: Signal<bool>,
    surface_switcher_sheet_visible: Signal<bool>,
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
    let close_chat = Callback::new(move |_| set_expanded.set(mobile_chat_after_close()));

    Effect::new(move |_| {
        if mobile_chat_runtime_conflict_should_close(
            visible.get(),
            drawer_open.get(),
            diff_open.get(),
            surface_switcher_sheet_visible.get(),
            expanded.get(),
        ) {
            set_expanded.set(mobile_chat_after_close());
        }
    });

    view! {
        <Show when=move || {
            should_show_mobile_chat_sheet(
                visible.get(),
                drawer_open.get(),
                diff_open.get(),
                surface_switcher_sheet_visible.get(),
                expanded.get(),
                keyboard_offset.get(),
            )
        }>
            <div
                class=move || mobile_chat_sheet_class(expanded.get())
                style=move || mobile_chat_sheet_style(expanded.get(), keyboard_offset.get())
                data-deve-mobile-chat=move || if expanded.get() { "expanded" } else { "collapsed" }
                data-deve-mobile-chat-page=move || mobile_chat_page_mode(expanded.get())
                data-deve-mobile-chat-fullscreen=move || expanded.get().to_string()
                data-deve-mobile-chat-keyboard-offset=move || keyboard_offset.get().to_string()
            >
                <Show
                    when=move || expanded.get()
                    fallback=move || {
                        view! {
                            <button
                                type="button"
                                data-deve-mobile-chat-action="mobile_chat_chip"
                                class="mobile-chat-chip h-11 min-w-[44px] px-3 rounded-full bg-panel border border-default shadow-sm text-sm font-medium text-primary active:bg-hover"
                                title=move || t::chat::toggle_mobile_chat(locale.get())
                                aria-label=move || t::chat::toggle_mobile_chat(locale.get())
                                on:click=move |_| set_expanded.set(mobile_chat_after_open())
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
mod tests;
