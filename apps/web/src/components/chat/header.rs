// apps/web/src/components/chat/header.rs
//! plan_ref:
//!   - 10_ai_agent#native-ai-chat-runtime
//!
use crate::api::AI_BACKEND_NATIVE;
use crate::components::chat::slash_commands::ChatSessionMode;
use crate::components::icons::*;
use crate::hooks::use_core::ChatContext;
use crate::i18n::{Locale, t};
use leptos::prelude::*;

#[component]
pub fn ChatHeader(
    #[prop(optional)] ai_mode: Option<ReadSignal<String>>,
    #[prop(optional)] session_mode: Option<ReadSignal<ChatSessionMode>>,
    #[prop(optional)] mobile: bool,
    on_close: Callback<()>,
) -> impl IntoView {
    let locale = use_context::<RwSignal<Locale>>().expect("locale context");
    let context_ai_mode = use_context::<ChatContext>().map(|chat| chat.ai_mode);
    let backend_label = move || {
        let mode = ai_mode
            .as_ref()
            .map(|signal| signal.get())
            .or_else(|| context_ai_mode.map(|signal| signal.get()))
            .unwrap_or_else(|| AI_BACKEND_NATIVE.to_string());
        if mode == AI_BACKEND_NATIVE {
            t::chat::ai_chat(locale.get())
        } else {
            t::chat::agent_bridge(locale.get())
        }
    };
    let mode_label = move || match session_mode
        .as_ref()
        .map(|signal| signal.get())
        .unwrap_or(ChatSessionMode::Plan)
    {
        ChatSessionMode::Plan => t::chat::mode_plan(locale.get()),
        ChatSessionMode::Build => t::chat::mode_build(locale.get()),
    };
    view! {
        <div class=move || if mobile {
            "h-12 flex items-center px-3 border-b border-default bg-panel"
        } else {
            "h-9 flex items-center px-4 border-b border-default bg-panel"
        } style=move || if mobile {
            "padding-top: env(safe-area-inset-top); height: calc(48px + env(safe-area-inset-top));"
        } else {
            ""
        }>
            <span class="text-xs font-bold text-primary uppercase tracking-wider">{move || t::chat::panel_title(locale.get())}</span>
            <span class="ml-2 text-[10px] font-mono px-2 py-[2px] rounded bg-badge-success text-badge-success border border-badge-success">
                {backend_label}
            </span>
            <span class="ml-1 text-[10px] font-mono px-2 py-[2px] rounded bg-hover text-secondary border border-default">
                {mode_label}
            </span>
            <div class="flex-1"></div>
            {move || if mobile {
                view! {
                    <button
                        class="chat-close-button h-11 min-w-[44px] rounded-md text-secondary active:bg-hover transition-colors duration-200 ease-out"
                        on:click=move |_| on_close.run(())
                        title=move || t::chat::toggle_mobile_chat(locale.get())
                        aria-label=move || t::chat::toggle_mobile_chat(locale.get())
                    >
                        <X class="w-4 h-4 mx-auto" />
                    </button>
                }
                    .into_any()
            } else {
                view! {}.into_any()
            }}
        </div>
    }
}
