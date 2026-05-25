// apps/web/src/components/settings_sections.rs
//! plan_ref:
//!   - 16_ai_agent#native-ai-chat-runtime
//!   - 16_ai_agent#trusted-agent-bridge
//!
//! # Settings Modal — Section Components
//!
//! Extracted sub-sections: Sync Mode, AI Backend.

use crate::api::{AI_BACKEND_NATIVE, AI_BACKEND_TRUSTED_CLI};
use crate::components::settings_sections_policy::{
    ai_backend_button_state, sync_mode_button_state,
};
use crate::hooks::use_ai_backend::use_ai_backend_capabilities_with_fallback;
use crate::i18n::{Locale, t};
use leptos::prelude::*;

/// Sync mode toggle (auto / manual).
#[component]
pub fn SyncModeSection(locale: RwSignal<Locale>) -> impl IntoView {
    move || {
        let core = expect_context::<crate::hooks::use_core::SyncMergeContext>();
        let button_state =
            Signal::derive(move || sync_mode_button_state(core.sync_mode.get().as_str()));
        view! {
            <div class="bg-sidebar p-4 rounded-lg border border-default flex justify-between items-center">
                <div>
                    <span class="font-medium text-primary">{move || t::settings::sync_mode(locale.get())}</span>
                    <p class="text-xs text-muted">{move || t::settings::sync_mode_desc(locale.get())}</p>
                </div>
                <div class="flex gap-2">
                    <button
                        class=move || button_state.get().auto_class
                        on:click=move |_| core.on_set_sync_mode.run("auto".to_string())
                    >
                        {move || t::settings::auto_mode(locale.get())}
                    </button>
                    <button
                        class=move || button_state.get().manual_class
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
        let trusted_cap = use_ai_backend_capabilities_with_fallback(chat.clone(), locale);
        let button_state = Signal::derive(move || {
            ai_backend_button_state(
                chat.ai_mode.get().as_str(),
                &trusted_cap.get(),
                locale.get(),
            )
        });
        view! {
            <div class="bg-sidebar p-4 rounded-lg border border-default flex justify-between items-center">
                <div>
                    <span class="font-medium text-primary">{move || t::settings::ai_backend(locale.get())}</span>
                    <p class="text-xs text-muted">{move || t::settings::ai_backend_desc(locale.get())}</p>
                </div>
                <div class="flex gap-2">
                    <button
                        class=move || button_state.get().native_class
                        disabled=move || button_state.get().native_disabled
                        aria-disabled=move || button_state.get().native_disabled.to_string()
                        title=move || button_state.get().native_title
                        on:click=move |_| {
                            if !button_state.get_untracked().native_disabled {
                                chat.set_ai_mode.set(AI_BACKEND_NATIVE.to_string());
                            }
                        }
                    >
                        {move || t::settings::native_backend(locale.get())}
                    </button>
                    <button
                        class=move || button_state.get().trusted_class
                        disabled=move || button_state.get().trusted_disabled
                        aria-disabled=move || button_state.get().trusted_disabled.to_string()
                        title=move || button_state.get().trusted_title
                        on:click=move |_| {
                            if !button_state.get_untracked().trusted_disabled {
                                chat.set_ai_mode.set(AI_BACKEND_TRUSTED_CLI.to_string());
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
