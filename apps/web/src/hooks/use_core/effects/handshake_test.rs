use super::handshake_state::reset_handshake_attempt_state;
use super::{
    handshake_mode_key, restore_bootstrap_key, should_restore_session_scope,
    should_suspend_handshake,
};
use crate::hooks::use_core::{PendingBranchTarget, types::HandshakeSignals};
use crate::storage::DegradedSyncMode;
use crate::storage::identity::StoredPeerIdentity;
use deve_core::models::{PeerId, VersionVector};
use leptos::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;

#[test]
fn suspends_handshake_while_viewing_shadow_branch() {
    assert!(should_suspend_handshake(
        &Some(PeerId::new("peer-a")),
        None,
        None,
    ));
}

#[test]
fn suspends_handshake_while_branch_switch_is_pending() {
    assert!(should_suspend_handshake(
        &None,
        Some(&PendingBranchTarget::Shadow("peer-a".into())),
        None,
    ));
}

#[test]
fn suspends_handshake_while_repo_switch_is_pending() {
    assert!(should_suspend_handshake(&None, None, Some("default")));
}

#[test]
fn keeps_handshake_enabled_for_local_bound_repo() {
    assert!(!should_suspend_handshake(&None, None, None));
}

#[test]
fn restore_runs_only_on_clean_reconnect_edge() {
    assert!(should_restore_session_scope(true, None, None));
    assert!(!should_restore_session_scope(
        true,
        Some(&PendingBranchTarget::Local),
        None,
    ));
    assert!(!should_restore_session_scope(true, None, Some("default")));
    assert!(!should_restore_session_scope(false, None, None));
}

#[test]
fn handshake_mode_key_distinguishes_local_and_shadow_scope() {
    let local = handshake_mode_key("ws://a", None, Some("repo-1"), None);
    let shadow = handshake_mode_key("ws://a", None, Some("repo-1"), Some(&PeerId::new("peer-a")));
    assert_ne!(local, shadow);
}

#[test]
fn restore_bootstrap_key_runs_once_per_unbound_scope() {
    let restore_key = restore_bootstrap_key("ws://a", None, None, 7, true, None)
        .expect("missing identity should still bootstrap session restore");
    assert_eq!(
        restore_bootstrap_key("ws://a", None, None, 7, true, Some(&restore_key)),
        None
    );
    assert_ne!(
        restore_key,
        restore_bootstrap_key("ws://a", Some("default"), None, 7, true, None)
            .expect("changing repo hint should reopen bootstrap")
    );
}

#[test]
fn restore_bootstrap_key_does_not_reopen_on_scope_nonce_churn() {
    let restore_key = restore_bootstrap_key("ws://a", Some("default"), None, 7, true, None)
        .expect("repo-bound reconnect should bootstrap once");
    assert_eq!(
        restore_bootstrap_key("ws://a", Some("default"), None, 8, true, Some(&restore_key)),
        None
    );
}

#[test]
fn reset_handshake_attempt_state_clears_retry_blockers() {
    let last_mode = Rc::new(RefCell::new(Some("ws://a::repo-1::local".to_string())));

    let (identity, _) = signal(None::<StoredPeerIdentity>);
    let (repo_vector, _) = signal(VersionVector::new());
    let (degraded, _) = signal(None::<DegradedSyncMode>);
    let (current_repo, _) = signal(Some("repo-1".to_string()));
    let (current_repo_id, _) = signal(Some(uuid::Uuid::new_v4().to_string()));
    let (current_scope_nonce, _) = signal(7u64);
    let (active_branch, _) = signal(None::<PeerId>);
    let (pending_branch_switch, set_pending_branch_switch) = signal(None::<PendingBranchTarget>);
    let (_pending_branch_switch_nonce, set_pending_branch_switch_nonce) = signal(None::<u64>);
    let (pending_repo_switch, set_pending_repo_switch) = signal(None::<String>);
    let (_pending_repo_switch_nonce, set_pending_repo_switch_nonce) = signal(None::<u64>);
    let (handshake_scope_nonce, set_handshake_scope_nonce) = signal(Some(7u64));
    let (repo_list_request_id, set_repo_list_request_id) = signal(Some("repo-1".to_string()));
    let (doc_list_request_id, set_doc_list_request_id) = signal(Some("doc-1".to_string()));
    let (tree_request_id, set_tree_request_id) = signal(Some("tree-1".to_string()));
    let (handshake_ready, set_handshake_ready) = signal(true);

    let signals = HandshakeSignals {
        identity,
        repo_vector,
        degraded,
        current_repo,
        current_repo_id,
        current_scope_nonce,
        active_branch,
        pending_branch_switch,
        set_pending_branch_switch,
        set_pending_branch_switch_nonce,
        pending_repo_switch,
        set_pending_repo_switch,
        set_pending_repo_switch_nonce,
        handshake_scope_nonce,
        set_handshake_scope_nonce,
        set_repo_list_request_id,
        set_doc_list_request_id,
        set_tree_request_id,
        set_handshake_ready,
    };

    reset_handshake_attempt_state(&last_mode, signals);

    assert!(last_mode.borrow().is_none());
    assert!(!handshake_ready.get_untracked());
    assert_eq!(handshake_scope_nonce.get_untracked(), None);
    assert_eq!(repo_list_request_id.get_untracked(), None);
    assert_eq!(doc_list_request_id.get_untracked(), None);
    assert_eq!(tree_request_id.get_untracked(), None);
}
