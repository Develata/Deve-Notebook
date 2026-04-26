use crate::api::AiBackendCapabilities;
use crate::hooks::use_core::{ChatContext, ChatMessage};
use crate::i18n::{Locale, t};
use leptos::prelude::*;

pub fn attach_trusted_cli_fallback(
    chat: ChatContext,
    capabilities: ReadSignal<AiBackendCapabilities>,
    locale: RwSignal<Locale>,
) {
    let last_notice = RwSignal::new(None::<String>);
    Effect::new(move |_| {
        let cap = capabilities.get();
        if cap.trusted_cli_available || chat.ai_mode.get() != "agent-bridge" {
            return;
        }

        chat.set_ai_mode.set("ai-chat".to_string());
        let reason = cap
            .trusted_cli_reason
            .unwrap_or_else(|| t::extensions::trusted_cli_unavailable(locale.get()).to_string());
        let notice = t::extensions::trusted_cli_fallback(locale.get(), &reason);
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
