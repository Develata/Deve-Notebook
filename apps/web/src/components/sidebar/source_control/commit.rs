// apps\web\src\components\source_control
//! # Commit Component (提交组件)
//!
//! VS Code 风格:
//! - Input Message Box
//! - Blue "Commit" button with dropdown arrow

use crate::hooks::use_core::CoreState;
use leptos::prelude::*;
use web_sys::KeyboardEvent;

#[component]
pub fn Commit() -> impl IntoView {
    let core = expect_context::<CoreState>();

    let (msg, set_msg) = signal(String::new());

    // 是否有暂存文件 (VS Code allows commiting all if none staged, but we keep it safe for now)
    let has_staged = move || !core.staged_changes.get().is_empty();

    let do_commit = move || {
        if !has_staged() || msg.get().trim().is_empty() {
            return;
        }
        core.on_commit.run(msg.get());
        set_msg.set(String::new());
    };

    let on_keydown = move |ev: KeyboardEvent| {
        if ev.ctrl_key() && ev.key() == "Enter" {
            do_commit();
        }
    };

    view! {
        <div class="px-2 pb-3 pt-1">
            <div class="flex flex-col gap-2">
                <div class="relative w-full">
                    <textarea
                        class="w-full h-9 p-1.5 pr-20 text-[13px] bg-white dark:bg-[#3c3c3c] border border-[#cecece] dark:border-[#3c3c3c] rounded-[2px] focus:outline-none focus:border-[#007fd4] focus:ring-1 focus:ring-[#007fd4] placeholder-gray-400 dark:text-[#cccccc] font-sans resize-none block leading-tight"
                        placeholder="消息(Ctrl+Enter 在“main”提交)"
                        prop:value=msg
                        on:input=move |ev| set_msg.set(event_target_value(&ev))
                        on:keydown=on_keydown
                        disabled=move || !has_staged()
                    />
                    <button
                        class="absolute right-1 top-1 bottom-1 px-1.5 bg-[#007fd4] hover:bg-[#006ab1] text-white text-[10px] rounded flex items-center gap-1 transition-colors z-10"
                        title="Generate Commit Message"
                        on:click=move |_| { /* Placeholder for AI generation */ }
                    >
                         <svg xmlns="http://www.w3.org/2000/svg" class="w-3 h-3" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M9.937 15.5A2 2 0 0 0 8.5 14.063l-6.135-1.582a.5.5 0 0 1 0-.962L8.5 9.936A2 2 0 0 0 9.937 8.5l1.582-6.135a.5.5 0 0 1 .963 0L14.063 8.5A2 2 0 0 0 15.5 9.937l6.135 1.581a.5.5 0 0 1 0 .964L15.5 14.063a2 2 0 0 0-1.437 1.437l-1.582 6.135a.5.5 0 0 1-.963 0z"/></svg>
                         "Generate"
                    </button>
                    // Sparkles icon might be better, using star for now or custom svg
                </div>

                <div class="flex">
                    <button
                        class="flex-1 bg-[#007fd4] hover:bg-[#006ab1] text-white text-[13px] font-medium py-1.5 rounded-l-[2px] flex items-center justify-center gap-1 disabled:opacity-50 disabled:bg-[#007fd4] disabled:cursor-not-allowed transition-colors shadow-sm"
                        disabled=move || !has_staged() || msg.get().trim().is_empty()
                        on:click=move |_| do_commit()
                    >
                        <span class="codicon codicon-check"></span>
                        <span>"提交"</span> // Chinese "Commit"
                    </button>
                    <button class="bg-[#007fd4] hover:bg-[#006ab1] text-white px-2 rounded-r-[2px] border-l border-[rgba(255,255,255,0.2)]">
                         <svg xmlns="http://www.w3.org/2000/svg" class="w-3.5 h-3.5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M6 9l6 6 6-6"/></svg>
                    </button>
                </div>
            </div>
        </div>
    }
}
