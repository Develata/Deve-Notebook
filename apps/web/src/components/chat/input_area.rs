// apps/web/src/components/chat/input_area.rs
use crate::i18n::{Locale, t};
use leptos::prelude::*;

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
            class=move || if mobile {
                "p-2 border-t border-[#e5e5e5] dark:border-[#252526] bg-white dark:bg-[#1e1e1e]"
            } else {
                "p-3 border-t border-[#e5e5e5] dark:border-[#252526] bg-white dark:bg-[#1e1e1e]"
            }
            style=move || if mobile { "padding-bottom: calc(8px + env(safe-area-inset-bottom));" } else { "" }
        >
            <div class="relative rounded border border-[#e5e5e5] dark:border-[#3e3e42] bg-white dark:bg-[#252526] focus-within:border-[#007acc] dark:focus-within:border-[#007acc] transition-colors">
                <textarea
                    name="ai-chat-input"
                    class="w-full max-h-32 p-2 bg-transparent border-none outline-none text-sm resize-none dark:text-[#cccccc] font-sans"
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
                    <span class="text-[10px] text-[#858585]">{move || t::chat::markdown_supported(locale.get())}</span>
                    <button
                        class=move || if mobile {
                            "h-11 min-w-11 p-2 rounded active:bg-[#f3f3f3] dark:active:bg-[#3e3e42] text-[#007acc] disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
                        } else {
                            "p-1.5 rounded hover:bg-[#f3f3f3] dark:hover:bg-[#3e3e42] text-[#007acc] disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
                        }
                        disabled=move || input.get().trim().is_empty() || is_streaming.get()
                        on:click=move |_| send_message.run(())
                        title=move || t::chat::send(locale.get())
                        aria-label=move || t::chat::send(locale.get())
                    >
                        <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                            <line x1="22" y1="2" x2="11" y2="13"></line>
                            <polygon points="22 2 15 22 11 13 2 9 22 2"></polygon>
                        </svg>
                    </button>
                </div>
            </div>
        </div>
    }
}
