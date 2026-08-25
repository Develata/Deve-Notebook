use super::super::message_dispatch_runtime::{
    handle_plugin_response_message, handle_search_results_message,
};
use super::{
    accepts_chat_chunk, accepts_plugin_response, accepts_search_results, accepts_unscoped_update,
};
use crate::api::ConnectionStatus;
use crate::hooks::use_core::state::init_signals;
use crate::hooks::use_core::types::ChatMessage;
use crate::hooks::use_core::{PendingBranchSwitch, PendingBranchTarget, PendingRepoSwitch};
use crate::i18n::Locale;
use crate::runtime::domain::SearchHit;
use deve_core::models::PeerId;
use leptos::prelude::*;

#[test]
fn rejects_unscoped_updates_while_repo_switch_pending() {
    let runtime = leptos::reactive::owner::Owner::new();
    runtime.set();
    let (connection_status, _) = signal(ConnectionStatus::Connected);
    let signals = init_signals(connection_status);
    signals
        .set_pending_repo_switch
        .set(Some(PendingRepoSwitch::switch(
            "test",
            uuid::Uuid::nil(),
            1,
        )));
    assert!(!accepts_unscoped_update(signals));
}

#[test]
fn rejects_unscoped_updates_while_branch_switch_pending() {
    let runtime = leptos::reactive::owner::Owner::new();
    runtime.set();
    let (connection_status, _) = signal(ConnectionStatus::Connected);
    let signals = init_signals(connection_status);
    signals
        .set_pending_branch_switch
        .set(Some(PendingBranchSwitch::new(
            PendingBranchTarget::Local,
            1,
        )));
    assert!(!accepts_unscoped_update(signals));
}

#[test]
fn rejects_search_results_when_request_id_is_stale() {
    let runtime = leptos::reactive::owner::Owner::new();
    runtime.set();
    let (connection_status, _) = signal(ConnectionStatus::Connected);
    let signals = init_signals(connection_status);
    let repo_id = uuid::Uuid::new_v4();
    signals.set_search_request_id.set(Some("fresh".into()));
    signals.set_current_scope_nonce.set(11);
    signals.set_current_repo_id.set(Some(repo_id.to_string()));
    assert!(!accepts_search_results(
        "stale",
        Some(repo_id),
        None,
        Some(11),
        signals
    ));
    assert!(!accepts_search_results(
        "fresh",
        Some(repo_id),
        None,
        None,
        signals
    ));
    assert!(!accepts_search_results(
        "fresh",
        Some(repo_id),
        None,
        Some(7),
        signals
    ));
    assert!(!accepts_search_results(
        "fresh",
        Some(uuid::Uuid::new_v4()),
        None,
        Some(11),
        signals
    ));
    signals.set_active_branch.set(Some(PeerId::new("remote")));
    assert!(!accepts_search_results(
        "fresh",
        Some(repo_id),
        None,
        Some(11),
        signals
    ));
    assert!(accepts_search_results(
        "fresh",
        Some(repo_id),
        Some(PeerId::new("remote")),
        Some(11),
        signals
    ));
}

#[test]
fn rejects_search_results_while_scope_switch_is_pending() {
    let runtime = leptos::reactive::owner::Owner::new();
    runtime.set();
    let (connection_status, _) = signal(ConnectionStatus::Connected);
    let signals = init_signals(connection_status);
    let repo_id = uuid::Uuid::new_v4();
    signals.set_search_request_id.set(Some("fresh".into()));
    signals.set_current_scope_nonce.set(11);
    signals.set_current_repo_id.set(Some(repo_id.to_string()));

    signals
        .set_pending_repo_switch
        .set(Some(PendingRepoSwitch::switch(
            "other",
            uuid::Uuid::nil(),
            1,
        )));
    assert!(!accepts_search_results(
        "fresh",
        Some(repo_id),
        None,
        Some(11),
        signals
    ));

    signals.set_pending_repo_switch.set(None);
    signals
        .set_pending_branch_switch
        .set(Some(PendingBranchSwitch::new(
            PendingBranchTarget::Local,
            1,
        )));
    assert!(!accepts_search_results(
        "fresh",
        Some(repo_id),
        None,
        Some(11),
        signals
    ));
}

#[test]
fn accepted_search_results_clear_pending_request() {
    let runtime = leptos::reactive::owner::Owner::new();
    runtime.set();
    let (connection_status, _) = signal(ConnectionStatus::Connected);
    let signals = init_signals(connection_status);
    let repo_id = uuid::Uuid::new_v4();
    signals.set_search_request_id.set(Some("search-1".into()));
    signals.set_current_scope_nonce.set(11);
    signals.set_current_repo_id.set(Some(repo_id.to_string()));
    handle_search_results_message(
        "search-1".into(),
        Some(repo_id),
        None,
        Some(11),
        vec![SearchHit::new("doc-1".into(), "notes/a.md".into(), 1.0)],
        signals,
    );

    assert_eq!(signals.search_request_id.get_untracked(), None);
    assert_eq!(signals.search_results.get_untracked().len(), 1);
    assert!(!accepts_search_results(
        "search-1",
        Some(repo_id),
        None,
        Some(11),
        signals
    ));
}

#[test]
fn rejects_plugin_response_when_req_id_is_stale() {
    let runtime = leptos::reactive::owner::Owner::new();
    runtime.set();
    let (connection_status, _) = signal(ConnectionStatus::Connected);
    let signals = init_signals(connection_status);
    signals.set_plugin_request_ids.set(vec!["fresh".into()]);
    assert!(!accepts_plugin_response("stale", signals));
    assert!(accepts_plugin_response("fresh", signals));
}

#[test]
fn accepts_chat_finish_for_existing_message_after_response_ack() {
    let runtime = leptos::reactive::owner::Owner::new();
    runtime.set();
    let (connection_status, _) = signal(ConnectionStatus::Connected);
    let signals = init_signals(connection_status);
    signals.set_chat_messages.set(vec![ChatMessage::new(
        "assistant",
        String::new(),
        Some("req-1".into()),
        0,
    )]);
    assert!(accepts_chat_chunk("req-1", signals));
    assert!(!accepts_chat_chunk("stale", signals));
}

#[test]
fn plugin_text_response_stops_loading() {
    let runtime = leptos::reactive::owner::Owner::new();
    runtime.set();
    let (connection_status, _) = signal(ConnectionStatus::Connected);
    let signals = init_signals(connection_status);
    signals.set_plugin_request_ids.set(vec!["req-1".into()]);
    signals.set_is_chat_streaming.set(true);
    signals.set_chat_messages.set(vec![ChatMessage::new(
        "assistant",
        String::new(),
        Some("req-1".into()),
        0,
    )]);

    handle_plugin_response_message(
        "req-1".into(),
        Some(serde_json::json!({"type": "text", "content": "Missing AI API key"})),
        None,
        Locale::En,
        signals,
    );

    assert!(!signals.is_chat_streaming.get_untracked());
    assert_eq!(
        signals.plugin_request_ids.get_untracked(),
        Vec::<String>::new()
    );
    assert_eq!(
        signals.chat_messages.get_untracked()[0].content,
        "Missing AI API key"
    );
}

#[test]
fn plugin_text_response_does_not_duplicate_streamed_chat_content() {
    let runtime = leptos::reactive::owner::Owner::new();
    runtime.set();
    let (connection_status, _) = signal(ConnectionStatus::Connected);
    let signals = init_signals(connection_status);
    signals.set_plugin_request_ids.set(vec!["req-1".into()]);
    signals.set_is_chat_streaming.set(true);
    signals.set_chat_messages.set(vec![ChatMessage::new(
        "assistant",
        "hello",
        Some("req-1".into()),
        0,
    )]);

    handle_plugin_response_message(
        "req-1".into(),
        Some(serde_json::json!({"type": "text", "content": "hello"})),
        None,
        Locale::En,
        signals,
    );

    assert!(!signals.is_chat_streaming.get_untracked());
    assert_eq!(signals.chat_messages.get_untracked()[0].content, "hello");
}
