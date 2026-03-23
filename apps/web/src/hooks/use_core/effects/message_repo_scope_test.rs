use super::{
    accepts_edit_rejected_message, accepts_protocol_error_message, accepts_write_ready,
    matches_repo_scope,
};
use crate::api::ConnectionStatus;
use crate::hooks::use_core::PendingBranchTarget;
use crate::hooks::use_core::state::init_signals;
use deve_core::models::PeerId;
use leptos::prelude::*;

#[test]
fn rejects_repo_scoped_messages_while_repo_switch_pending() {
    let runtime = leptos::reactive::owner::Owner::new();
    runtime.set();
    let repo_id = uuid::Uuid::new_v4();
    let (current_repo_id, _) = signal(Some(repo_id.to_string()));
    assert!(!matches_repo_scope(
        &Some(repo_id),
        &None,
        current_repo_id,
        None,
        None,
        Some("test".into()),
    ));
}

#[test]
fn rejects_repo_scoped_messages_while_branch_switch_pending() {
    let runtime = leptos::reactive::owner::Owner::new();
    runtime.set();
    let repo_id = uuid::Uuid::new_v4();
    let (current_repo_id, _) = signal(Some(repo_id.to_string()));
    assert!(!matches_repo_scope(
        &Some(repo_id),
        &Some(PeerId::new("peer-a")),
        current_repo_id,
        None,
        Some(PendingBranchTarget::Shadow("peer-a".into())),
        None,
    ));
}

#[test]
fn rejects_write_ready_while_repo_switch_pending() {
    let repo_id = uuid::Uuid::new_v4().to_string();
    assert!(!accepts_write_ready(
        &repo_id,
        &None,
        3,
        Some(repo_id.clone()),
        None,
        Some(PendingBranchTarget::Local),
        Some("default".into()),
        Some(3),
    ));
}

#[test]
fn rejects_write_ready_while_branch_switch_pending() {
    let repo_id = uuid::Uuid::new_v4().to_string();
    assert!(!accepts_write_ready(
        &repo_id,
        &None,
        3,
        Some(repo_id.clone()),
        None,
        Some(PendingBranchTarget::Local),
        None,
        Some(3),
    ));
}

#[test]
fn accepts_write_ready_only_for_local_branch_and_bound_repo() {
    let repo_id = uuid::Uuid::new_v4().to_string();
    assert!(accepts_write_ready(
        &repo_id,
        &None,
        3,
        Some(repo_id.clone()),
        None,
        None,
        None,
        Some(3),
    ));
    assert!(!accepts_write_ready(
        &repo_id,
        &Some(PeerId::new("peer-a")),
        3,
        Some(repo_id.clone()),
        Some(PeerId::new("peer-a")),
        None,
        None,
        Some(3),
    ));
    assert!(!accepts_write_ready(
        &repo_id,
        &None,
        2,
        Some(repo_id.clone()),
        None,
        None,
        None,
        Some(3),
    ));
}

#[test]
fn rejects_edit_rejected_while_scope_switch_pending_or_nonce_is_stale() {
    let runtime = leptos::reactive::owner::Owner::new();
    runtime.set();
    let (status, _) = signal(ConnectionStatus::Connected);
    let signals = init_signals(status);
    signals.set_current_scope_nonce.set(7);
    signals
        .set_pending_branch_switch
        .set(Some(PendingBranchTarget::Local));
    assert!(!accepts_edit_rejected_message(Some(7), signals));
    signals.set_pending_branch_switch.set(None);
    signals.set_pending_repo_switch.set(Some("wiki".into()));
    assert!(!accepts_edit_rejected_message(Some(7), signals));
    signals.set_pending_repo_switch.set(None);
    assert!(!accepts_edit_rejected_message(Some(9), signals));
    assert!(!accepts_edit_rejected_message(None, signals));
    assert!(accepts_edit_rejected_message(Some(7), signals));
}

#[test]
fn rejects_scoped_protocol_errors_while_scope_switch_pending_or_nonce_is_stale() {
    let runtime = leptos::reactive::owner::Owner::new();
    runtime.set();
    let (status, _) = signal(ConnectionStatus::Connected);
    let signals = init_signals(status);
    signals.set_current_scope_nonce.set(7);
    signals
        .set_pending_branch_switch
        .set(Some(PendingBranchTarget::Local));
    assert!(!accepts_protocol_error_message(Some(7), None, signals));
    signals.set_pending_branch_switch.set(None);
    signals.set_pending_repo_switch.set(Some("wiki".into()));
    assert!(!accepts_protocol_error_message(Some(7), None, signals));
    signals.set_pending_repo_switch.set(None);
    assert!(!accepts_protocol_error_message(Some(9), None, signals));
    assert!(!accepts_protocol_error_message(None, None, signals));
    assert!(!accepts_protocol_error_message(None, Some(3), signals));
    signals.set_pending_repo_switch_nonce.set(Some(3));
    assert!(accepts_protocol_error_message(None, Some(3), signals));
    assert!(accepts_protocol_error_message(Some(7), None, signals));
}
