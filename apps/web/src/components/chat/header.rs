// apps/web/src/components/chat/header.rs
use crate::i18n::{Locale, t};
use leptos::prelude::*;

#[component]
pub fn ChatHeader(
    ai_mode: ReadSignal<String>,
    #[prop(optional)] mobile: bool,
    on_close: Callback<()>,
) -> impl IntoView {
    let locale = use_context::<RwSignal<Locale>>().expect("locale context");
    view! {
        <div class=move || if mobile {
            "h-12 flex items-center px-3 border-b border-[#e5e5e5] dark:border-[#252526] bg-[#f8f8f8] dark:bg-[#2d2d2d]"
        } else {
            "h-9 flex items-center px-4 border-b border-[#e5e5e5] dark:border-[#252526] bg-[#f8f8f8] dark:bg-[#2d2d2d]"
        } style=move || if mobile {
            "padding-top: env(safe-area-inset-top); height: calc(48px + env(safe-area-inset-top));"
        } else {
            ""
        }>
            <span class="text-xs font-bold text-[#3b3b3b] dark:text-[#cccccc] uppercase tracking-wider">{move || t::chat::panel_title(locale.get())}</span>
            <span class="ml-2 text-[10px] uppercase font-mono px-2 py-[2px] rounded bg-[#eeeeee] dark:bg-[#3a3a3a] text-[#555555] dark:text-[#cccccc] border border-[#dddddd] dark:border-[#4a4a4a]">
                {move || if ai_mode.get() == "plan" {
                    t::chat::mode_plan(locale.get())
                } else {
                    t::chat::mode_build(locale.get())
                }}
            </span>
            <div class="flex-1"></div>
            <select
                name="ai-model-select"
                class=move || if mobile {
                "hidden"
            } else {
                "text-xs bg-transparent border-none outline-none text-[#616161] dark:text-[#858585] cursor-pointer"
            }
            >
                <option>"GPT-4o"</option>
                <option>"Claude 3.5"</option>
                <option>"DeepSeek V3"</option>
            </select>
            {move || if mobile {
                view! {
                    <button
                        class="chat-close-button h-11 min-w-11 rounded-md text-gray-600 active:bg-gray-200 transition-colors duration-200 ease-out"
                        on:click=move |_| on_close.run(())
                        title=move || t::chat::toggle_mobile_chat(locale.get())
                        aria-label=move || t::chat::toggle_mobile_chat(locale.get())
                    >
                        <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 20 20" fill="none" stroke="currentColor" stroke-width="2" class="w-4 h-4 mx-auto">
                            <path stroke-linecap="round" stroke-linejoin="round" d="M6 6l8 8M14 6l-8 8" />
                        </svg>
                    </button>
                }
                    .into_any()
            } else {
                view! {}.into_any()
            }}
        </div>
    }
}
