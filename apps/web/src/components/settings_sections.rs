// apps/web/src/components/settings_sections.rs
//! plan_ref:
//!   - 15_settings#browser-ui-prefs
//!   - 16_ai_agent#native-ai-chat-runtime
//!   - 16_ai_agent#trusted-agent-bridge
//!
//! # Settings Modal — Section Components
//!
//! Extracted sub-sections: local preferences, sync mode, AI backend.

use crate::api::{AI_BACKEND_NATIVE, AI_BACKEND_TRUSTED_CLI};
use crate::components::settings_sections_policy::{
    ai_backend_button_state, sync_mode_button_state,
};
use crate::hooks::use_ai_backend::use_ai_backend_capabilities_with_fallback;
use crate::i18n::{Locale, t};
use leptos::prelude::*;

mod local_prefs;

pub use local_prefs::{AppearanceSection, EditorBasicsSection, RuntimeDiagnosticsSection};

/// Sync mode toggle (auto / manual).
#[component]
pub fn SyncModeSection(locale: RwSignal<Locale>) -> impl IntoView {
    move || {
        let core = expect_context::<crate::hooks::use_core::SyncMergeContext>();
        let button_state =
            Signal::derive(move || sync_mode_button_state(core.sync_mode.get().as_str()));
        view! {
            <div class="bg-sidebar p-4 rounded-lg border border-default flex flex-col gap-3 sm:flex-row sm:items-center sm:justify-between">
                <div>
                    <span class="font-medium text-primary">{move || t::settings::sync_mode(locale.get())}</span>
                    <p class="text-xs text-muted">{move || t::settings::sync_mode_desc(locale.get())}</p>
                </div>
                <div class="flex flex-wrap gap-2">
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
        let disabled_reason = Signal::derive(move || {
            let state = button_state.get();
            if state.native_disabled && !state.native_title.is_empty() {
                state.native_title
            } else if state.trusted_disabled && !state.trusted_title.is_empty() {
                state.trusted_title
            } else {
                String::new()
            }
        });
        view! {
            <div class="bg-sidebar p-4 rounded-lg border border-default">
                <div class="flex flex-col gap-3 sm:flex-row sm:items-center sm:justify-between">
                    <div>
                        <span class="font-medium text-primary">{move || t::settings::ai_backend(locale.get())}</span>
                        <p class="text-xs text-muted">{move || t::settings::ai_backend_desc(locale.get())}</p>
                    </div>
                    <div class="flex flex-wrap gap-2">
                        <button
                            class=move || button_state.get().native_class
                            disabled=move || button_state.get().native_disabled
                            aria-disabled=move || button_state.get().native_disabled.to_string()
                            data-deve-setting-disabled=move || button_state.get().native_disabled.to_string()
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
                            data-deve-setting-disabled=move || button_state.get().trusted_disabled.to_string()
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
                <Show when=move || !disabled_reason.get().is_empty()>
                    <p class="mt-3 text-xs text-accent" data-deve-setting-disabled-reason="ai_backend">
                        {move || disabled_reason.get()}
                    </p>
                </Show>
            </div>
        }
    }
}
