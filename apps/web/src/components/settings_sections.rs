// apps/web/src/components/settings_sections.rs
//! plan_ref:
//!   - 10_ai_agent#native-ai-chat-runtime
//!   - 10_ai_agent#trusted-agent-bridge
//!
//! # Settings Modal — Section Components
//!
//! Extracted sub-sections: Sync Mode, AI Backend.

use crate::api::{
    AI_BACKEND_NATIVE, AI_BACKEND_TRUSTED_CLI, AiBackendCapabilities, fetch_ai_backend_capabilities,
};
use crate::components::ai_backend_guard::attach_trusted_cli_fallback;
use crate::components::settings_sections_policy::{
    ai_backend_button_state, sync_mode_button_state,
};
use crate::i18n::{Locale, t};
use leptos::prelude::*;
use leptos::task::spawn_local;

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
        let (trusted_cap, set_trusted_cap) = signal(AiBackendCapabilities::default());
        Effect::new(move |_| {
            let set_trusted_cap = set_trusted_cap;
            spawn_local(async move {
                set_trusted_cap.set(fetch_ai_backend_capabilities().await);
            });
        });
        attach_trusted_cli_fallback(chat.clone(), trusted_cap, locale);
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
