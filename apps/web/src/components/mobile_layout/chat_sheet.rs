// apps/web/src/components/mobile_layout/chat_sheet.rs

use crate::components::chat::ChatPanel;
use crate::components::layout_context::ChatControl;
use crate::i18n::{Locale, t};
use leptos::prelude::*;

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
        <Show when=move || visible.get() && !drawer_open.get() && !diff_open.get() && keyboard_offset.get() <= 0>
            <div
                class=move || if expanded.get() {
                    "fixed inset-0 z-[80] bg-white dark:bg-[#1e1e1e] transition-opacity duration-200 ease-out"
                } else {
                    "fixed right-2 z-[55]"
                }
                style=move || {
                    let base = if keyboard_offset.get() > 0 {
                        keyboard_offset.get()
                    } else {
                        58
                    };
                    if expanded.get() {
                        "".to_string()
                    } else {
                        format!("bottom: calc({}px + env(safe-area-inset-bottom));", base)
                    }
                }
            >
                <Show
                    when=move || expanded.get()
                    fallback=move || {
                        view! {
                            <button
                                class="mobile-chat-chip h-11 min-w-11 px-3 rounded-full bg-white border border-gray-200 shadow-sm text-sm font-medium text-gray-700 active:bg-gray-100"
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
