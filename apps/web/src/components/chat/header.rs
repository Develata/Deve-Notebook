// apps/web/src/components/chat/header.rs
use leptos::prelude::*;

#[component]
pub fn ChatHeader(ai_mode: ReadSignal<String>) -> impl IntoView {
    view! {
        <div class="h-9 flex items-center px-4 border-b border-[#e5e5e5] dark:border-[#252526] bg-[#f8f8f8] dark:bg-[#2d2d2d]">
            <span class="text-xs font-bold text-[#3b3b3b] dark:text-[#cccccc] uppercase tracking-wider">"AI Assistant"</span>
            <span class="ml-2 text-[10px] uppercase font-mono px-2 py-[2px] rounded bg-[#eeeeee] dark:bg-[#3a3a3a] text-[#555555] dark:text-[#cccccc] border border-[#dddddd] dark:border-[#4a4a4a]">
                {move || if ai_mode.get() == "plan" { "PLAN" } else { "BUILD" }}
            </span>
            <div class="flex-1"></div>
            <select class="text-xs bg-transparent border-none outline-none text-[#616161] dark:text-[#858585] cursor-pointer">
                <option>"GPT-4o"</option>
                <option>"Claude 3.5"</option>
                <option>"DeepSeek V3"</option>
            </select>
        </div>
    }
}
