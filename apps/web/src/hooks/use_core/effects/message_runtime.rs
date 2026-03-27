use crate::hooks::use_core::state::CoreSignals;
use leptos::prelude::{Set, Update};

pub fn handle_chat_chunk(
    req_id: String,
    delta: Option<String>,
    finish_reason: Option<String>,
    signals: CoreSignals,
) {
    if let Some(delta) = delta.filter(|text| !text.is_empty()) {
        signals.set_chat_messages.update(|messages| {
            if let Some(existing) = messages
                .iter_mut()
                .rev()
                .find(|msg| msg.req_id.as_deref() == Some(req_id.as_str()))
            {
                existing.content.push_str(&delta);
            }
        });
    }

    if finish_reason.is_some() {
        signals.set_is_chat_streaming.set(false);
    }
}
#[cfg(test)]
mod tests {
    use super::handle_chat_chunk;
    use crate::hooks::use_core::state_init::init_signals;
    use leptos::prelude::*;

    #[test]
    fn chat_chunk_ignores_unknown_req_after_scope_reset() {
        let runtime = leptos::reactive::owner::Owner::new();
        runtime.set();
        let signals = init_signals(signal(crate::api::ConnectionStatus::Disconnected).0);
        signals.set_is_chat_streaming.set(true);

        handle_chat_chunk("req-1".into(), Some("late".into()), None, signals);

        assert!(signals.chat_messages.get_untracked().is_empty());
    }

    #[test]
    fn chat_chunk_appends_to_matching_message() {
        let runtime = leptos::reactive::owner::Owner::new();
        runtime.set();
        let signals = init_signals(signal(crate::api::ConnectionStatus::Disconnected).0);
        signals
            .set_chat_messages
            .set(vec![crate::hooks::use_core::types::ChatMessage {
                role: "assistant".into(),
                content: "hello".into(),
                req_id: Some("req-1".into()),
                ts_ms: 1,
            }]);

        handle_chat_chunk("req-1".into(), Some(" world".into()), None, signals);

        assert_eq!(
            signals.chat_messages.get_untracked()[0].content,
            "hello world"
        );
    }
}
