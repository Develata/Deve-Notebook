//! plan_ref:
//!   - 10_ai_agent#native-ai-chat-runtime
//!
use crate::api::{AI_BACKEND_NATIVE, AI_BACKEND_TRUSTED_CLI, AiBackendCapabilities};
use crate::hooks::use_core::{ChatContext, ChatMessage};
use crate::i18n::{Locale, t};
use leptos::prelude::*;

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

pub fn attach_trusted_cli_fallback(
    chat: ChatContext,
    capabilities: ReadSignal<AiBackendCapabilities>,
    locale: RwSignal<Locale>,
) {
    let last_notice = RwSignal::new(None::<String>);
    Effect::new(move |_| {
        let cap = capabilities.get();
        let fallback = select_backend_fallback(chat.ai_mode.get().as_str(), &cap, locale.get());
        let reason = match fallback {
            BackendFallback::Switch { backend, reason } => {
                chat.set_ai_mode.set(backend.to_string());
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
        chat.set_messages.update(|messages| {
            messages.push(ChatMessage {
                role: "assistant".to_string(),
                content: notice,
                req_id: None,
                ts_ms: js_sys::Date::now() as u64,
            });
        });
    });
}

fn select_backend_fallback(
    current_backend: &str,
    cap: &AiBackendCapabilities,
    locale: Locale,
) -> BackendFallback {
    if current_backend == AI_BACKEND_TRUSTED_CLI && !cap.trusted_cli_available {
        let reason = cap
            .trusted_cli_reason
            .clone()
            .unwrap_or_else(|| t::extensions::trusted_cli_unavailable(locale).to_string());
        if cap.native_available {
            return BackendFallback::Switch {
                backend: AI_BACKEND_NATIVE,
                reason,
            };
        }
        return BackendFallback::Blocked { reason };
    }

    if current_backend == AI_BACKEND_NATIVE && !cap.native_available {
        let reason = cap
            .native_reason
            .clone()
            .or_else(|| cap.effective_backend_reason.clone())
            .unwrap_or_else(|| "native AI disabled by config".to_string());
        if cap.trusted_cli_available {
            return BackendFallback::Switch {
                backend: AI_BACKEND_TRUSTED_CLI,
                reason,
            };
        }
        return BackendFallback::Blocked { reason };
    }

    if current_backend != cap.effective_backend {
        match cap.effective_backend.as_str() {
            AI_BACKEND_NATIVE if cap.native_available => {
                return BackendFallback::Switch {
                    backend: AI_BACKEND_NATIVE,
                    reason: cap
                        .effective_backend_reason
                        .clone()
                        .unwrap_or_else(|| "effective backend is native".to_string()),
                };
            }
            AI_BACKEND_TRUSTED_CLI if cap.trusted_cli_available => {
                return BackendFallback::Switch {
                    backend: AI_BACKEND_TRUSTED_CLI,
                    reason: cap
                        .effective_backend_reason
                        .clone()
                        .unwrap_or_else(|| "effective backend is trusted-cli".to_string()),
                };
            }
            _ => {}
        }
    }

    BackendFallback::Keep
}

#[cfg(test)]
mod tests {
    use super::{BackendFallback, select_backend_fallback};
    use crate::api::{AI_BACKEND_NATIVE, AI_BACKEND_TRUSTED_CLI, AiBackendCapabilities};
    use crate::i18n::Locale;

    #[test]
    fn trusted_cli_falls_back_to_native_when_policy_blocks_it() {
        let cap = AiBackendCapabilities {
            native_available: true,
            trusted_cli_available: false,
            trusted_cli_reason: Some("external agent disabled".to_string()),
            ..AiBackendCapabilities::default()
        };

        assert_eq!(
            select_backend_fallback(AI_BACKEND_TRUSTED_CLI, &cap, Locale::En),
            BackendFallback::Switch {
                backend: AI_BACKEND_NATIVE,
                reason: "external agent disabled".to_string()
            }
        );
    }

    #[test]
    fn native_falls_back_to_trusted_cli_when_native_is_disabled_and_trusted_is_available() {
        let cap = AiBackendCapabilities {
            native_available: false,
            native_reason: Some("native AI disabled by config".to_string()),
            trusted_cli_available: true,
            ..AiBackendCapabilities::default()
        };

        assert_eq!(
            select_backend_fallback(AI_BACKEND_NATIVE, &cap, Locale::En),
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
            select_backend_fallback(AI_BACKEND_NATIVE, &cap, Locale::En),
            BackendFallback::Blocked {
                reason: "native AI disabled by config".to_string()
            }
        );
    }
}
