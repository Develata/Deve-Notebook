//! plan_ref:
//!   - 15_settings#browser-ui-prefs
//!   - 16_ai_agent#trusted-agent-bridge
//!
//! Pure Settings section UI policy helpers.

use crate::api::{AI_BACKEND_NATIVE, AI_BACKEND_TRUSTED_CLI, AiBackendCapabilities};
use crate::components::settings_prefs::{
    EditorDensityPreference, EditorWrapPreference, ThemePreference,
};
use crate::i18n::{Locale, t};

pub(super) const BUTTON_CLASS_DISABLED: &str =
    "min-h-[44px] px-3 py-2 text-sm font-medium text-muted rounded opacity-50 cursor-not-allowed";
pub(super) const BUTTON_CLASS_IDLE: &str = "min-h-[44px] px-3 py-2 text-sm font-medium text-muted hover:bg-active rounded transition-colors";
const BUTTON_CLASS_ACCENT_ACTIVE: &str =
    "min-h-[44px] px-3 py-2 text-sm font-bold bg-accent text-on-accent rounded transition-colors";
const SYNC_AUTO_CLASS_ACTIVE: &str =
    "min-h-[44px] px-3 py-2 text-sm font-bold bg-green-500 text-white rounded transition-colors";
const SYNC_MANUAL_CLASS_ACTIVE: &str =
    "min-h-[44px] px-3 py-2 text-sm font-bold bg-yellow-500 text-white rounded transition-colors";

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct LanguageButtonState {
    pub english_class: &'static str,
    pub chinese_class: &'static str,
}

pub(super) fn language_button_state(locale: Locale) -> LanguageButtonState {
    LanguageButtonState {
        english_class: if locale == Locale::En {
            BUTTON_CLASS_ACCENT_ACTIVE
        } else {
            BUTTON_CLASS_IDLE
        },
        chinese_class: if locale == Locale::Zh {
            BUTTON_CLASS_ACCENT_ACTIVE
        } else {
            BUTTON_CLASS_IDLE
        },
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct ReservedSettingState {
    pub class: &'static str,
    pub disabled_attr: &'static str,
    pub aria_disabled: &'static str,
    pub reason: String,
}

pub(super) fn reserved_setting_state(locale: Locale) -> ReservedSettingState {
    ReservedSettingState {
        class: "bg-sidebar p-4 rounded-lg border border-default opacity-80 cursor-not-allowed select-none",
        disabled_attr: "true",
        aria_disabled: "true",
        reason: t::settings::coming_soon(locale).to_string(),
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct SyncModeButtonState {
    pub auto_class: &'static str,
    pub manual_class: &'static str,
}

pub(super) fn sync_mode_button_state(sync_mode: &str) -> SyncModeButtonState {
    let is_manual = sync_mode == "manual";
    SyncModeButtonState {
        auto_class: if is_manual {
            BUTTON_CLASS_IDLE
        } else {
            SYNC_AUTO_CLASS_ACTIVE
        },
        manual_class: if is_manual {
            SYNC_MANUAL_CLASS_ACTIVE
        } else {
            BUTTON_CLASS_IDLE
        },
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct ThemeButtonState {
    pub warm_class: &'static str,
    pub cold_class: &'static str,
    pub night_class: &'static str,
}

pub(super) fn theme_button_state(pref: ThemePreference) -> ThemeButtonState {
    ThemeButtonState {
        warm_class: preference_button_class(pref == ThemePreference::Warm),
        cold_class: preference_button_class(pref == ThemePreference::Cold),
        night_class: preference_button_class(pref == ThemePreference::Night),
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct EditorWrapButtonState {
    pub on_class: &'static str,
    pub off_class: &'static str,
}

pub(super) fn editor_wrap_button_state(pref: EditorWrapPreference) -> EditorWrapButtonState {
    EditorWrapButtonState {
        on_class: preference_button_class(pref == EditorWrapPreference::On),
        off_class: preference_button_class(pref == EditorWrapPreference::Off),
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct EditorDensityButtonState {
    pub comfortable_class: &'static str,
    pub compact_class: &'static str,
}

pub(super) fn editor_density_button_state(
    pref: EditorDensityPreference,
) -> EditorDensityButtonState {
    EditorDensityButtonState {
        comfortable_class: preference_button_class(pref == EditorDensityPreference::Comfortable),
        compact_class: preference_button_class(pref == EditorDensityPreference::Compact),
    }
}

fn preference_button_class(active: bool) -> &'static str {
    if active {
        BUTTON_CLASS_ACCENT_ACTIVE
    } else {
        BUTTON_CLASS_IDLE
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct AiChatVisibilityButtonState {
    pub show_class: &'static str,
    pub hide_class: &'static str,
}

pub(super) fn ai_chat_visibility_button_state(visible: bool) -> AiChatVisibilityButtonState {
    AiChatVisibilityButtonState {
        show_class: preference_button_class(visible),
        hide_class: preference_button_class(!visible),
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct NativeBackendButtonState {
    pub local_class: &'static str,
    pub remote_class: &'static str,
}

pub(super) fn native_backend_button_state(mode: &str) -> NativeBackendButtonState {
    NativeBackendButtonState {
        local_class: preference_button_class(mode != "remote"),
        remote_class: preference_button_class(mode == "remote"),
    }
}

pub(super) fn native_backend_can_switch_local(mode: &str) -> bool {
    mode == "remote"
}

pub(super) fn native_backend_validation_state(
    busy: bool,
    feedback: &str,
    mode: &str,
    remote_validation_succeeded: bool,
) -> &'static str {
    if busy {
        "pending"
    } else if mode != "remote" || feedback.is_empty() {
        "idle"
    } else if remote_validation_succeeded {
        "success"
    } else {
        "failed"
    }
}

pub(super) fn native_backend_unavailable_feedback(locale: Locale, feedback: &str) -> String {
    let feedback = feedback.trim();
    let error = if feedback.is_empty() {
        None
    } else {
        Some(feedback)
    };
    t::settings::native_backend_error(locale, error)
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct AiBackendButtonState {
    pub native_class: &'static str,
    pub native_disabled: bool,
    pub native_title: String,
    pub trusted_class: &'static str,
    pub trusted_disabled: bool,
    pub trusted_title: String,
}

pub(super) fn ai_backend_button_state(
    selected_backend: &str,
    capabilities: &AiBackendCapabilities,
    locale: Locale,
) -> AiBackendButtonState {
    let native_disabled = !capabilities.native_available;
    let trusted_disabled = !capabilities.trusted_cli_available;
    let native_selected = selected_backend == AI_BACKEND_NATIVE;
    let trusted_selected = selected_backend == AI_BACKEND_TRUSTED_CLI;

    AiBackendButtonState {
        native_class: ai_backend_button_class(native_disabled, native_selected),
        native_disabled,
        native_title: if native_disabled {
            capabilities
                .native_reason
                .as_deref()
                .map(|reason| t::extensions::ai_backend_reason(locale, reason))
                .unwrap_or_else(|| {
                    t::extensions::ai_backend_reason(locale, "native AI disabled by config")
                })
        } else {
            String::new()
        },
        trusted_class: ai_backend_button_class(trusted_disabled, trusted_selected),
        trusted_disabled,
        trusted_title: if trusted_disabled {
            capabilities
                .trusted_cli_reason
                .as_deref()
                .map(|reason| t::extensions::ai_backend_reason(locale, reason))
                .unwrap_or_else(|| t::extensions::trusted_cli_unavailable(locale).to_string())
        } else {
            String::new()
        },
    }
}

fn ai_backend_button_class(disabled: bool, selected: bool) -> &'static str {
    if disabled {
        BUTTON_CLASS_DISABLED
    } else if selected {
        BUTTON_CLASS_ACCENT_ACTIVE
    } else {
        BUTTON_CLASS_IDLE
    }
}

#[cfg(test)]
mod tests;
