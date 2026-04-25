// apps/web/src/components/settings_sections.rs
//! plan_ref:
//!   - 10_ai_agent#native-ai-chat-runtime
//!   - 10_ai_agent#trusted-agent-bridge
//!
//! # Settings Modal — Section Components
//!
//! Extracted sub-sections: Sync Mode, AI Backend.

use crate::api::{AiBackendCapabilities, fetch_ai_backend_capabilities};
use crate::i18n::{Locale, t};
use leptos::prelude::*;
use leptos::task::spawn_local;

/// Sync mode toggle (auto / manual).
#[component]
pub fn SyncModeSection(locale: RwSignal<Locale>) -> impl IntoView {
    move || {
        let core = expect_context::<crate::hooks::use_core::SyncMergeContext>();
        let is_manual = core.sync_mode.get() == "manual";
        view! {
            <div class="bg-sidebar p-4 rounded-lg border border-default flex justify-between items-center">
                <div>
                    <span class="font-medium text-primary">{move || t::settings::sync_mode(locale.get())}</span>
                    <p class="text-xs text-muted">{move || t::settings::sync_mode_desc(locale.get())}</p>
                </div>
                <div class="flex gap-2">
                    <button
                        class=move || {
                            if !is_manual {
                                "px-3 py-1 text-xs font-bold bg-green-500 text-white rounded transition-colors"
                            } else {
                                "px-3 py-1 text-xs font-medium text-muted hover:bg-active rounded transition-colors"
                            }
                        }
                        on:click=move |_| core.on_set_sync_mode.run("auto".to_string())
                    >
                        {move || t::settings::auto_mode(locale.get())}
                    </button>
                    <button
                        class=move || {
                            if is_manual {
                                "px-3 py-1 text-xs font-bold bg-yellow-500 text-white rounded transition-colors"
                            } else {
                                "px-3 py-1 text-xs font-medium text-muted hover:bg-active rounded transition-colors"
                            }
                        }
                        on:click=move |_| core.on_set_sync_mode.run("manual".to_string())
                    >
                        {move || t::settings::manual_mode(locale.get())}
                    </button>
                </div>
            </div>
        }
    }
}

/// AI backend toggle (Native / Trusted CLI).
#[component]
pub fn AiBackendSection(locale: RwSignal<Locale>) -> impl IntoView {
    move || {
        let chat = expect_context::<crate::hooks::use_core::ChatContext>();
        let (trusted_cap, set_trusted_cap) = signal(AiBackendCapabilities::default());
        Effect::new(move |_| {
            let set_trusted_cap = set_trusted_cap;
            spawn_local(async move {
                set_trusted_cap.set(fetch_ai_backend_capabilities().await);
            });
        });
        let is_native = Signal::derive(move || chat.ai_mode.get() == "ai-chat");
        let trusted_available = Signal::derive(move || trusted_cap.get().trusted_cli_available);
        let trusted_reason = move || {
            trusted_cap
                .get()
                .trusted_cli_reason
                .unwrap_or_else(|| t::extensions::trusted_cli_unavailable(locale.get()).to_string())
        };
        view! {
            <div class="bg-sidebar p-4 rounded-lg border border-default flex justify-between items-center">
                <div>
                    <span class="font-medium text-primary">{move || t::settings::ai_backend(locale.get())}</span>
                    <p class="text-xs text-muted">{move || t::settings::ai_backend_desc(locale.get())}</p>
                </div>
                <div class="flex gap-2">
                    <button
                        class=move || if is_native.get() {
                            "px-3 py-1 text-xs font-bold bg-accent text-on-accent rounded transition-colors"
                        } else {
                            "px-3 py-1 text-xs font-medium text-muted hover:bg-active rounded transition-colors"
                        }
                        on:click=move |_| chat.set_ai_mode.set("ai-chat".to_string())
                    >
                        {move || t::settings::native_backend(locale.get())}
                    </button>
                    <button
                        class=move || if !trusted_available.get() {
                            "px-3 py-1 text-xs font-medium text-muted rounded opacity-50 cursor-not-allowed"
                        } else if !is_native.get() {
                            "px-3 py-1 text-xs font-bold bg-accent text-on-accent rounded transition-colors"
                        } else {
                            "px-3 py-1 text-xs font-medium text-muted hover:bg-active rounded transition-colors"
                        }
                        disabled=move || !trusted_available.get()
                        title=trusted_reason
                        on:click=move |_| {
                            if trusted_available.get_untracked() {
                                chat.set_ai_mode.set("agent-bridge".to_string());
                            }
                        }
                    >
                        {move || t::settings::trusted_cli_backend(locale.get())}
                    </button>
                </div>
            </div>
        }
    }
}
