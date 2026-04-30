//! plan_ref:
//!   - 13_settings#browser-ui-prefs
//!   - 10_ai_agent#trusted-agent-bridge
//!
//! Pure Settings section UI policy helpers.

use crate::api::{AI_BACKEND_NATIVE, AI_BACKEND_TRUSTED_CLI, AiBackendCapabilities};
use crate::i18n::{Locale, t};

pub(super) const BUTTON_CLASS_DISABLED: &str =
    "px-3 py-1 text-xs font-medium text-muted rounded opacity-50 cursor-not-allowed";
pub(super) const BUTTON_CLASS_IDLE: &str =
    "px-3 py-1 text-xs font-medium text-muted hover:bg-active rounded transition-colors";
const AI_BACKEND_CLASS_ACTIVE: &str =
    "px-3 py-1 text-xs font-bold bg-accent text-on-accent rounded transition-colors";
const SYNC_AUTO_CLASS_ACTIVE: &str =
    "px-3 py-1 text-xs font-bold bg-green-500 text-white rounded transition-colors";
const SYNC_MANUAL_CLASS_ACTIVE: &str =
    "px-3 py-1 text-xs font-bold bg-yellow-500 text-white rounded transition-colors";

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
                .clone()
                .unwrap_or_else(|| "Native AI disabled by config".to_string())
        } else {
            String::new()
        },
        trusted_class: ai_backend_button_class(trusted_disabled, trusted_selected),
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

fn ai_backend_button_class(disabled: bool, selected: bool) -> &'static str {
    if disabled {
        BUTTON_CLASS_DISABLED
    } else if selected {
        AI_BACKEND_CLASS_ACTIVE
    } else {
        BUTTON_CLASS_IDLE
    }
}

#[cfg(test)]
mod tests {
    use super::{
        AI_BACKEND_CLASS_ACTIVE, BUTTON_CLASS_DISABLED, BUTTON_CLASS_IDLE, SYNC_AUTO_CLASS_ACTIVE,
        SYNC_MANUAL_CLASS_ACTIVE, ai_backend_button_state, sync_mode_button_state,
    };
    use crate::api::{AI_BACKEND_NATIVE, AI_BACKEND_TRUSTED_CLI, AiBackendCapabilities};
    use crate::i18n::Locale;

    #[test]
    fn sync_mode_buttons_reflect_current_mode() {
        let auto = sync_mode_button_state("auto");
        assert_eq!(auto.auto_class, SYNC_AUTO_CLASS_ACTIVE);
        assert_eq!(auto.manual_class, BUTTON_CLASS_IDLE);

        let manual = sync_mode_button_state("manual");
        assert_eq!(manual.auto_class, BUTTON_CLASS_IDLE);
        assert_eq!(manual.manual_class, SYNC_MANUAL_CLASS_ACTIVE);
    }

    #[test]
    fn sync_mode_buttons_treat_unknown_mode_as_auto_safe_default() {
        let state = sync_mode_button_state("unexpected");
        assert_eq!(state.auto_class, SYNC_AUTO_CLASS_ACTIVE);
        assert_eq!(state.manual_class, BUTTON_CLASS_IDLE);
    }

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

        assert_eq!(state.native_class, AI_BACKEND_CLASS_ACTIVE);
        assert!(!state.native_disabled);
        assert!(state.native_title.is_empty());
        assert_eq!(state.trusted_class, BUTTON_CLASS_DISABLED);
        assert!(state.trusted_disabled);
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

        assert_eq!(state.native_class, BUTTON_CLASS_DISABLED);
        assert!(state.native_disabled);
        assert_eq!(state.native_title, "native AI disabled by config");
        assert_eq!(state.trusted_class, BUTTON_CLASS_IDLE);
        assert!(!state.trusted_disabled);
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

        assert_eq!(state.native_class, BUTTON_CLASS_IDLE);
        assert!(!state.native_disabled);
        assert_eq!(state.trusted_class, AI_BACKEND_CLASS_ACTIVE);
        assert!(!state.trusted_disabled);
        assert!(state.native_title.is_empty());
        assert!(state.trusted_title.is_empty());
    }
}
