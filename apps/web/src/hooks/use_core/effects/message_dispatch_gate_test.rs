use super::{
    accepts_chat_chunk, accepts_plugin_response, accepts_search_results, accepts_unscoped_update,
};
use crate::api::ConnectionStatus;
use crate::hooks::use_core::PendingBranchTarget;
use crate::hooks::use_core::state::init_signals;
use crate::hooks::use_core::types::ChatMessage;
use deve_core::models::PeerId;
use leptos::prelude::*;

#[test]
fn rejects_unscoped_updates_while_repo_switch_pending() {
    let runtime = leptos::reactive::owner::Owner::new();
    runtime.set();
    let (connection_status, _) = signal(ConnectionStatus::Connected);
    let signals = init_signals(connection_status);
    signals.set_pending_repo_switch.set(Some("test".into()));
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
        .set(Some(PendingBranchTarget::Local));
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
    signals.set_chat_messages.set(vec![ChatMessage {
        role: "assistant".into(),
        content: String::new(),
        req_id: Some("req-1".into()),
        ts_ms: 0,
    }]);
    assert!(accepts_chat_chunk("req-1", signals));
    assert!(!accepts_chat_chunk("stale", signals));
}
