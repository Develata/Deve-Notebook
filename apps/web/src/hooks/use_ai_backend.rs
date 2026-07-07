//! plan_ref:
//!   - 16_ai_agent#native-ai-chat-runtime
//!   - 16_ai_agent#trusted-agent-bridge
//!
//! Reactive AI backend capability state and fallback effects.

use crate::api::{
    AiBackendCapabilities, BackendSendDecision, fetch_ai_backend_capabilities,
    resolve_backend_for_effective_state,
};
use crate::i18n::{Locale, t};
use crate::runtime::domain::{AiBackendMode, ChatMessage};
use leptos::prelude::*;
use leptos::task::spawn_local;

#[derive(Debug, PartialEq, Eq)]
enum BackendFallback {
    Switch {
        backend: &'static str,
        reason: String,
    },
    Blocked {
        reason: String,
    },
    Keep,
}

#[derive(Clone, Copy)]
pub struct AiBackendFallbackSignals {
    pub ai_mode: ReadSignal<AiBackendMode>,
    pub set_ai_mode: WriteSignal<AiBackendMode>,
    pub set_messages: WriteSignal<Vec<ChatMessage>>,
}

pub fn use_ai_backend_capabilities_with_fallback(
    signals: AiBackendFallbackSignals,
    locale: RwSignal<Locale>,
) -> ReadSignal<AiBackendCapabilities> {
    let capabilities = use_ai_backend_capabilities();
    attach_trusted_cli_fallback(signals, capabilities, locale);
    capabilities
}

fn use_ai_backend_capabilities() -> ReadSignal<AiBackendCapabilities> {
    let (capabilities, set_capabilities) = signal(AiBackendCapabilities::default());
    Effect::new(move |_| {
        spawn_local(async move {
            set_capabilities.set(fetch_ai_backend_capabilities().await);
        });
    });
    capabilities
}

fn attach_trusted_cli_fallback(
    signals: AiBackendFallbackSignals,
    capabilities: ReadSignal<AiBackendCapabilities>,
    locale: RwSignal<Locale>,
) {
    let last_notice = RwSignal::new(None::<String>);
    Effect::new(move |_| {
        let cap = capabilities.get();
        let fallback = select_backend_fallback(signals.ai_mode.get().as_str(), &cap);
        let reason = match fallback {
            BackendFallback::Switch { backend, reason } => {
                signals
                    .set_ai_mode
                    .set(AiBackendMode::from_backend_str_or_native(backend));
                reason
            }
            BackendFallback::Blocked { reason } => reason,
            BackendFallback::Keep => return,
        };
        let notice = t::extensions::ai_backend_fallback(locale.get(), &reason);
        if last_notice.get_untracked().as_deref() == Some(notice.as_str()) {
            return;
        }
        last_notice.set(Some(notice.clone()));
        signals.set_messages.update(|messages| {
            if latest_message_is_notice(messages, &notice) {
                return;
            }
            messages.push(ChatMessage {
                role: "assistant".to_string(),
                content: notice,
                req_id: None,
                ts_ms: js_sys::Date::now() as u64,
            });
        });
    });
}

fn latest_message_is_notice(messages: &[ChatMessage], notice: &str) -> bool {
    matches!(
        messages.last(),
        Some(message)
            if message.role == "assistant"
                && message.req_id.is_none()
                && message.content == notice
    )
}

fn select_backend_fallback(current_backend: &str, cap: &AiBackendCapabilities) -> BackendFallback {
    match resolve_backend_for_effective_state(current_backend, cap) {
        BackendSendDecision::Use(_) => BackendFallback::Keep,
        BackendSendDecision::Switch { backend, reason } => {
            BackendFallback::Switch { backend, reason }
        }
        BackendSendDecision::Block { reason } => BackendFallback::Blocked { reason },
    }
}

#[cfg(test)]
mod tests {
    use super::{BackendFallback, latest_message_is_notice, select_backend_fallback};
    use crate::api::{AI_BACKEND_NATIVE, AI_BACKEND_TRUSTED_CLI, AiBackendCapabilities};
    use crate::runtime::domain::ChatMessage;

    #[test]
    fn trusted_cli_falls_back_to_native_when_policy_blocks_it() {
        let cap = AiBackendCapabilities {
            native_available: true,
            trusted_cli_available: false,
            trusted_cli_reason: Some("external agent disabled".to_string()),
            ..AiBackendCapabilities::default()
        };

        assert_eq!(
            select_backend_fallback(AI_BACKEND_TRUSTED_CLI, &cap),
            BackendFallback::Switch {
                backend: AI_BACKEND_NATIVE,
                reason: "external agent disabled".to_string()
            }
        );
    }

    #[test]
    fn native_does_not_auto_promote_to_trusted_cli_when_native_is_disabled() {
        let cap = AiBackendCapabilities {
            native_available: false,
            native_reason: Some("native AI disabled by config".to_string()),
            trusted_cli_available: true,
            effective_backend: "none".to_string(),
            ..AiBackendCapabilities::default()
        };

        assert_eq!(
            select_backend_fallback(AI_BACKEND_NATIVE, &cap),
            BackendFallback::Blocked {
                reason: "native AI disabled by config".to_string()
            }
        );
    }

    #[test]
    fn native_switches_to_trusted_cli_only_when_server_effective_backend_requests_it() {
        let cap = AiBackendCapabilities {
            native_available: false,
            native_reason: Some("native AI disabled by config".to_string()),
            trusted_cli_available: true,
            effective_backend: AI_BACKEND_TRUSTED_CLI.to_string(),
            effective_backend_reason: Some("trusted-cli explicitly requested".to_string()),
            ..AiBackendCapabilities::default()
        };

        assert_eq!(
            select_backend_fallback(AI_BACKEND_NATIVE, &cap),
            BackendFallback::Switch {
                backend: AI_BACKEND_TRUSTED_CLI,
                reason: "native AI disabled by config".to_string()
            }
        );
    }

    #[test]
    fn reports_blocked_when_no_ai_backend_is_available() {
        let cap = AiBackendCapabilities {
            native_available: false,
            native_reason: Some("native AI disabled by config".to_string()),
            trusted_cli_available: false,
            trusted_cli_reason: Some("external agent disabled".to_string()),
            effective_backend: "none".to_string(),
            effective_backend_reason: Some("no AI backend available".to_string()),
        };

        assert_eq!(
            select_backend_fallback(AI_BACKEND_NATIVE, &cap),
            BackendFallback::Blocked {
                reason: "native AI disabled by config".to_string()
            }
        );
    }

    #[test]
    fn suppresses_duplicate_fallback_notice_when_multiple_surfaces_mount() {
        let notice = "trusted-cli fallback";
        let messages = vec![ChatMessage {
            role: "assistant".to_string(),
            content: notice.to_string(),
            req_id: None,
            ts_ms: 1,
        }];

        assert!(latest_message_is_notice(&messages, notice));
    }

    #[test]
    fn allows_fallback_notice_after_user_message() {
        let notice = "trusted-cli fallback";
        let messages = vec![
            ChatMessage {
                role: "assistant".to_string(),
                content: notice.to_string(),
                req_id: None,
                ts_ms: 1,
            },
            ChatMessage {
                role: "user".to_string(),
                content: "try again".to_string(),
                req_id: None,
                ts_ms: 2,
            },
        ];

        assert!(!latest_message_is_notice(&messages, notice));
    }

    #[test]
    fn plugin_response_with_same_content_is_not_treated_as_fallback_notice() {
        let notice = "trusted-cli fallback";
        let messages = vec![ChatMessage {
            role: "assistant".to_string(),
            content: notice.to_string(),
            req_id: Some("plugin-response".to_string()),
            ts_ms: 1,
        }];

        assert!(!latest_message_is_notice(&messages, notice));
    }
}
