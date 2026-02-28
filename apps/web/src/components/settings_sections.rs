// apps/web/src/components/settings_sections.rs
//! # Settings Modal — Section Components
//!
//! Extracted sub-sections: Sync Mode, AI Backend.

use crate::i18n::{Locale, t};
use leptos::prelude::*;

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

/// AI backend toggle (CLI / API).
#[component]
pub fn AiBackendSection(locale: RwSignal<Locale>) -> impl IntoView {
    move || {
        let chat = expect_context::<crate::hooks::use_core::ChatContext>();
        let is_api = chat.ai_mode.get() == "ai-chat";
        view! {
            <div class="bg-sidebar p-4 rounded-lg border border-default flex justify-between items-center">
                <div>
                    <span class="font-medium text-primary">{move || t::settings::ai_backend(locale.get())}</span>
                    <p class="text-xs text-muted">{move || t::settings::ai_backend_desc(locale.get())}</p>
                </div>
                <div class="flex gap-2">
                    <button
                        class=move || if !is_api {
                            "px-3 py-1 text-xs font-bold bg-accent text-on-accent rounded transition-colors"
                        } else {
                            "px-3 py-1 text-xs font-medium text-muted hover:bg-active rounded transition-colors"
                        }
                        on:click=move |_| chat.set_ai_mode.set("agent-bridge".to_string())
                    >"CLI"</button>
                    <button
                        class=move || if is_api {
                            "px-3 py-1 text-xs font-bold bg-accent text-on-accent rounded transition-colors"
                        } else {
                            "px-3 py-1 text-xs font-medium text-muted hover:bg-active rounded transition-colors"
                        }
                        on:click=move |_| chat.set_ai_mode.set("ai-chat".to_string())
                    >"API"</button>
                </div>
            </div>
        }
    }
}
