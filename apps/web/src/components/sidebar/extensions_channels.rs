//! plan_ref:
//!   - 10_ai_agent#native-ai-chat-runtime
//!   - 10_ai_agent#trusted-agent-bridge
//!
use crate::api::{AiBackendCapabilities, fetch_ai_backend_capabilities};
use crate::components::icons::{Terminal, Zap};
use crate::hooks::use_core::ChatContext;
use crate::i18n::{Locale, t};
use leptos::prelude::*;
use leptos::task::spawn_local;

#[component]
pub fn AiChannelCards(locale: RwSignal<Locale>, chat: ChatContext) -> impl IntoView {
    let (trusted_cap, set_trusted_cap) = signal(AiBackendCapabilities::default());
    Effect::new(move |_| {
        let set_trusted_cap = set_trusted_cap;
        spawn_local(async move {
            set_trusted_cap.set(fetch_ai_backend_capabilities().await);
        });
    });

    let trusted_available = Signal::derive(move || trusted_cap.get().trusted_cli_available);
    let trusted_reason = move || {
        trusted_cap
            .get()
            .trusted_cli_reason
            .unwrap_or_else(|| t::extensions::trusted_cli_unavailable(locale.get()).to_string())
    };

    view! {
        <div class="space-y-3">
            <button
                class=move || if chat.ai_mode.get() == "ai-chat" {
                    "w-full rounded-xl border border-accent bg-accent/10 p-4 text-left"
                } else {
                    "w-full rounded-xl border border-default bg-panel hover:bg-active p-4 text-left transition-colors"
                }
                on:click=move |_| chat.set_ai_mode.set("ai-chat".to_string())
            >
                <div class="flex items-start justify-between gap-3">
                    <div class="flex gap-3">
                        <div class="rounded-lg bg-active p-2 text-primary"><Zap class="w-5 h-5" /></div>
                        <div>
                            <div class="text-sm font-semibold text-primary">{move || t::chat::ai_chat(locale.get())}</div>
                            <p class="mt-1 text-xs text-muted">{move || t::extensions::channel_desc(locale.get(), "ai-chat")}</p>
                        </div>
                    </div>
                    <span class="rounded-full border border-default px-2 py-1 text-[10px] font-medium text-secondary">
                        {move || t::extensions::status_label(locale.get(), chat.ai_mode.get() == "ai-chat")}
                    </span>
                </div>
            </button>
            <button
                class=move || if !trusted_available.get() {
                    "w-full rounded-xl border border-default bg-panel p-4 text-left opacity-50 cursor-not-allowed"
                } else if chat.ai_mode.get() == "agent-bridge" {
                    "w-full rounded-xl border border-accent bg-accent/10 p-4 text-left"
                } else {
                    "w-full rounded-xl border border-default bg-panel hover:bg-active p-4 text-left transition-colors"
                }
                disabled=move || !trusted_available.get()
                title=trusted_reason
                on:click=move |_| {
                    if trusted_available.get_untracked() {
                        chat.set_ai_mode.set("agent-bridge".to_string());
                    }
                }
            >
                <div class="flex items-start justify-between gap-3">
                    <div class="flex gap-3">
                        <div class="rounded-lg bg-active p-2 text-primary"><Terminal class="w-5 h-5" /></div>
                        <div>
                            <div class="text-sm font-semibold text-primary">{move || t::chat::agent_bridge(locale.get())}</div>
                            <p class="mt-1 text-xs text-muted">{move || t::extensions::channel_desc(locale.get(), "agent-bridge")}</p>
                        </div>
                    </div>
                    <span class="rounded-full border border-default px-2 py-1 text-[10px] font-medium text-secondary">
                        {move || t::extensions::trusted_status_label(locale.get(), chat.ai_mode.get() == "agent-bridge", trusted_available.get())}
                    </span>
                </div>
            </button>
        </div>
    }
}
