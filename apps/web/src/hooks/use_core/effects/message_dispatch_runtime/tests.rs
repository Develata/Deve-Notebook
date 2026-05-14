use super::handle_plugin_response_message;
use crate::api::ConnectionStatus;
use crate::hooks::use_core::state::init_signals;
use crate::hooks::use_core::types::ChatMessage;
use crate::i18n::Locale;
use deve_core::protocol::{ServerError, ServerErrorCode};
use leptos::prelude::*;
use leptos::reactive::owner::Owner;

fn init_chat_error_case(content: &str) -> (Owner, crate::hooks::use_core::state::CoreSignals) {
    let runtime = Owner::new();
    runtime.set();
    let (connection_status, _) = signal(ConnectionStatus::Connected);
    let signals = init_signals(connection_status);
    signals.set_plugin_request_ids.set(vec!["req-1".into()]);
    signals.set_is_chat_streaming.set(true);
    signals.set_chat_messages.set(vec![ChatMessage {
        role: "assistant".into(),
        content: content.into(),
        req_id: Some("req-1".into()),
        ts_ms: 0,
    }]);
    (runtime, signals)
}

#[test]
fn plugin_error_response_finishes_empty_chat_placeholder() {
    let (_runtime, signals) = init_chat_error_case("");

    handle_plugin_response_message(
        "req-1".into(),
        None,
        Some(ServerError::with_detail(
            ServerErrorCode::RequestFailed,
            "Native AI Chat tools are disabled by default",
        )),
        Locale::En,
        signals,
    );

    assert!(!signals.is_chat_streaming.get_untracked());
    assert_eq!(
        signals.chat_messages.get_untracked()[0].content,
        "Request failed"
    );
}

#[test]
fn plugin_error_response_appends_after_partial_streamed_content() {
    let (_runtime, signals) = init_chat_error_case("partial answer");

    handle_plugin_response_message(
        "req-1".into(),
        None,
        Some(ServerError::with_detail(
            ServerErrorCode::RequestFailed,
            "provider tool calls are disabled",
        )),
        Locale::En,
        signals,
    );

    assert!(!signals.is_chat_streaming.get_untracked());
    assert_eq!(
        signals.chat_messages.get_untracked()[0].content,
        "partial answer\n\nRequest failed"
    );
}
