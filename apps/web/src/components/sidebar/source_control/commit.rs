// apps\web\src\components\source_control
//! # Commit Component (提交组件)
//!
//! VS Code 风格:
//! - Input Message Box
//! - Blue "Commit" button with dropdown arrow

use crate::components::icons::*;
use crate::hooks::use_core::SourceControlContext;
use crate::i18n::{Locale, t};
use leptos::prelude::*;
use web_sys::KeyboardEvent;

#[component]
pub fn Commit() -> impl IntoView {
    let core = expect_context::<SourceControlContext>();
    let locale = use_context::<RwSignal<Locale>>().unwrap_or_else(|| RwSignal::new(Locale::En));

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
                        name="commit-message"
                        class="w-full h-9 p-1.5 pr-20 text-[13px] bg-input border border-default rounded-[2px] focus:outline-none focus:border-b-accent focus:ring-1 focus:ring-accent placeholder:text-muted text-primary font-sans resize-none block leading-tight"
                        placeholder=move || t::source_control::commit_message_placeholder(locale.get())
                        prop:value=msg
                        on:input=move |ev| set_msg.set(event_target_value(&ev))
                        on:keydown=on_keydown
                        disabled=move || !has_staged()
                    />
                    <button
                        class="absolute right-1 top-1 bottom-1 px-1.5 bg-accent hover:bg-accent-hover text-on-accent text-[10px] rounded flex items-center gap-1 transition-colors z-10"
                        title=move || t::source_control::generate_commit_message(locale.get())
                        on:click=move |_| { /* Placeholder for AI generation */ }
                    >
                         <Sparkles class="w-3 h-3" />
                         {move || t::source_control::generate(locale.get())}
                    </button>
                    // Sparkles icon might be better, using star for now or custom svg
                </div>

                <div class="flex">
                    <button
                        class="flex-1 bg-accent hover:bg-accent-hover text-on-accent text-[13px] font-medium py-1.5 rounded-l-[2px] flex items-center justify-center gap-1 disabled:opacity-50 disabled:bg-accent disabled:cursor-not-allowed transition-colors shadow-sm"
                        disabled=move || !has_staged() || msg.get().trim().is_empty()
                        on:click=move |_| do_commit()
                    >
                        <span class="codicon codicon-check"></span>
                        <span>{move || t::source_control::commit(locale.get())}</span>
                    </button>
                    <button class="bg-accent hover:bg-accent-hover text-on-accent px-2 rounded-r-[2px] border-l border-white/20">
                         <ChevronDown class="w-3.5 h-3.5" />
                    </button>
                </div>
            </div>
        </div>
    }
}
