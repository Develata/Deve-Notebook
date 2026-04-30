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
use crate::i18n::{Locale, t};
use leptos::prelude::*;
use leptos::task::spawn_local;

const AI_BACKEND_CLASS_DISABLED: &str =
    "px-3 py-1 text-xs font-medium text-muted rounded opacity-50 cursor-not-allowed";
const AI_BACKEND_CLASS_ACTIVE: &str =
    "px-3 py-1 text-xs font-bold bg-accent text-on-accent rounded transition-colors";
const AI_BACKEND_CLASS_IDLE: &str =
    "px-3 py-1 text-xs font-medium text-muted hover:bg-active rounded transition-colors";

#[derive(Clone, Debug, PartialEq, Eq)]
struct AiBackendButtonState {
    native_class: &'static str,
    native_disabled: bool,
    native_title: String,
    trusted_class: &'static str,
    trusted_disabled: bool,
    trusted_title: String,
}

fn ai_backend_button_state(
    selected_backend: &str,
    capabilities: &AiBackendCapabilities,
    locale: Locale,
) -> AiBackendButtonState {
    let native_disabled = !capabilities.native_available;
    let trusted_disabled = !capabilities.trusted_cli_available;
    let native_selected = selected_backend == AI_BACKEND_NATIVE;
    let trusted_selected = selected_backend == AI_BACKEND_TRUSTED_CLI;

    AiBackendButtonState {
        native_class: backend_button_class(native_disabled, native_selected),
        native_disabled,
        native_title: if native_disabled {
            capabilities
                .native_reason
                .clone()
                .unwrap_or_else(|| "Native AI disabled by config".to_string())
        } else {
            String::new()
        },
        trusted_class: backend_button_class(trusted_disabled, trusted_selected),
        trusted_disabled,
        trusted_title: if trusted_disabled {
            capabilities
                .trusted_cli_reason
                .clone()
                .unwrap_or_else(|| t::extensions::trusted_cli_unavailable(locale).to_string())
        } else {
            String::new()
        },
    }
}

fn backend_button_class(disabled: bool, selected: bool) -> &'static str {
    if disabled {
        AI_BACKEND_CLASS_DISABLED
    } else if selected {
        AI_BACKEND_CLASS_ACTIVE
    } else {
        AI_BACKEND_CLASS_IDLE
    }
}

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
        let native_available = Signal::derive(move || trusted_cap.get().native_available);
        let trusted_available = Signal::derive(move || trusted_cap.get().trusted_cli_available);
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
                        disabled=move || !native_available.get()
                        title=move || button_state.get().native_title
                        on:click=move |_| {
                            if native_available.get_untracked() {
                                chat.set_ai_mode.set(AI_BACKEND_NATIVE.to_string());
                            }
                        }
                    >
                        {move || t::settings::native_backend(locale.get())}
                    </button>
                    <button
                        class=move || button_state.get().trusted_class
                        disabled=move || !trusted_available.get()
                        title=move || button_state.get().trusted_title
                        on:click=move |_| {
                            if trusted_available.get_untracked() {
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

#[cfg(test)]
mod tests {
    use super::{
        AI_BACKEND_CLASS_ACTIVE, AI_BACKEND_CLASS_DISABLED, AI_BACKEND_CLASS_IDLE,
        ai_backend_button_state,
    };
    use crate::api::{AI_BACKEND_NATIVE, AI_BACKEND_TRUSTED_CLI, AiBackendCapabilities};
    use crate::i18n::Locale;

    #[test]
    fn ai_backend_buttons_disable_only_unavailable_backends() {
        let state = ai_backend_button_state(
            AI_BACKEND_NATIVE,
            &AiBackendCapabilities {
                native_available: true,
                trusted_cli_available: false,
                trusted_cli_reason: Some("external agent disabled".to_string()),
                ..AiBackendCapabilities::default()
            },
            Locale::En,
        );

        assert!(!state.native_disabled);
        assert_eq!(state.native_class, AI_BACKEND_CLASS_ACTIVE);
        assert!(state.native_title.is_empty());
        assert!(state.trusted_disabled);
        assert_eq!(state.trusted_class, AI_BACKEND_CLASS_DISABLED);
        assert_eq!(state.trusted_title, "external agent disabled");
    }

    #[test]
    fn ai_backend_buttons_show_disabled_reason_only_for_disabled_native() {
        let state = ai_backend_button_state(
            AI_BACKEND_NATIVE,
            &AiBackendCapabilities {
                native_available: false,
                native_reason: Some("native AI disabled by config".to_string()),
                trusted_cli_available: true,
                ..AiBackendCapabilities::default()
            },
            Locale::En,
        );

        assert!(state.native_disabled);
        assert_eq!(state.native_class, AI_BACKEND_CLASS_DISABLED);
        assert_eq!(state.native_title, "native AI disabled by config");
        assert!(!state.trusted_disabled);
        assert_eq!(state.trusted_class, AI_BACKEND_CLASS_IDLE);
        assert!(state.trusted_title.is_empty());
    }

    #[test]
    fn ai_backend_buttons_mark_trusted_cli_active_when_selected_and_available() {
        let state = ai_backend_button_state(
            AI_BACKEND_TRUSTED_CLI,
            &AiBackendCapabilities {
                native_available: true,
                trusted_cli_available: true,
                ..AiBackendCapabilities::default()
            },
            Locale::En,
        );

        assert_eq!(state.native_class, AI_BACKEND_CLASS_IDLE);
        assert_eq!(state.trusted_class, AI_BACKEND_CLASS_ACTIVE);
        assert!(state.native_title.is_empty());
        assert!(state.trusted_title.is_empty());
    }
}
