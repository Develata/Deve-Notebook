//! plan_ref:
//!   - 16_ai_agent#native-ai-chat-runtime
//!   - 16_ai_agent#trusted-agent-bridge
//!
use crate::api::{
    AI_BACKEND_NATIVE, AI_BACKEND_TRUSTED_CLI, AI_PLUGIN_NATIVE, AI_PLUGIN_TRUSTED_CLI,
};
use crate::components::icons::{Terminal, Zap};
use crate::hooks::use_ai_backend::use_ai_backend_capabilities_with_fallback;
use crate::hooks::use_core::{AiBackendMode, ChatContext};
use crate::i18n::{Locale, t};
use leptos::prelude::*;

#[component]
pub fn AiChannelCards(locale: RwSignal<Locale>, chat: ChatContext) -> impl IntoView {
    let trusted_cap = use_ai_backend_capabilities_with_fallback(chat.clone(), locale);
    let trusted_available = Signal::derive(move || trusted_cap.get().trusted_cli_available);
    let native_available = Signal::derive(move || trusted_cap.get().native_available);
    let native_reason = move || {
        trusted_cap
            .get()
            .native_reason
            .unwrap_or_else(|| "Native AI disabled by config".to_string())
    };
    let trusted_reason = move || {
        trusted_cap
            .get()
            .trusted_cli_reason
            .unwrap_or_else(|| t::extensions::trusted_cli_unavailable(locale.get()).to_string())
    };

    view! {
        <div class="space-y-3">
            <button
                class=move || if !native_available.get() {
                    "w-full rounded-xl border border-default bg-panel p-4 text-left opacity-50 cursor-not-allowed"
                } else if chat.ai_mode.get() == AI_BACKEND_NATIVE {
                    "w-full rounded-xl border border-accent bg-accent/10 p-4 text-left"
                } else {
                    "w-full rounded-xl border border-default bg-panel hover:bg-active p-4 text-left transition-colors"
                }
                disabled=move || !native_available.get()
                aria-disabled=move || (!native_available.get()).to_string()
                title=native_reason
                on:click=move |_| {
                    if native_available.get_untracked() {
                        chat.set_ai_mode.set(AiBackendMode::Native);
                    }
                }
            >
                <div class="flex items-start justify-between gap-3">
                    <div class="flex gap-3">
                        <div class="rounded-lg bg-active p-2 text-primary"><Zap class="w-5 h-5" /></div>
                        <div>
                            <div class="text-sm font-semibold text-primary">{move || t::chat::ai_chat(locale.get())}</div>
                            <p class="mt-1 text-xs text-muted">{move || t::extensions::channel_desc(locale.get(), AI_PLUGIN_NATIVE)}</p>
                        </div>
                    </div>
                    <span class="rounded-full border border-default px-2 py-1 text-[10px] font-medium text-secondary">
                        {move || t::extensions::status_label(locale.get(), chat.ai_mode.get() == AI_BACKEND_NATIVE)}
                    </span>
                </div>
            </button>
            <button
                class=move || if !trusted_available.get() {
                    "w-full rounded-xl border border-default bg-panel p-4 text-left opacity-50 cursor-not-allowed"
                } else if chat.ai_mode.get() == AI_BACKEND_TRUSTED_CLI {
                    "w-full rounded-xl border border-accent bg-accent/10 p-4 text-left"
                } else {
                    "w-full rounded-xl border border-default bg-panel hover:bg-active p-4 text-left transition-colors"
                }
                disabled=move || !trusted_available.get()
                aria-disabled=move || (!trusted_available.get()).to_string()
                title=trusted_reason
                on:click=move |_| {
                    if trusted_available.get_untracked() {
                        chat.set_ai_mode.set(AiBackendMode::TrustedCli);
                    }
                }
            >
                <div class="flex items-start justify-between gap-3">
                    <div class="flex gap-3">
                        <div class="rounded-lg bg-active p-2 text-primary"><Terminal class="w-5 h-5" /></div>
                        <div>
                            <div class="text-sm font-semibold text-primary">{move || t::chat::agent_bridge(locale.get())}</div>
                            <p class="mt-1 text-xs text-muted">{move || t::extensions::channel_desc(locale.get(), AI_PLUGIN_TRUSTED_CLI)}</p>
                        </div>
                    </div>
                    <span class="rounded-full border border-default px-2 py-1 text-[10px] font-medium text-secondary">
                        {move || t::extensions::trusted_status_label(locale.get(), chat.ai_mode.get() == AI_BACKEND_TRUSTED_CLI, trusted_available.get())}
                    </span>
                </div>
            </button>
        </div>
    }
}
