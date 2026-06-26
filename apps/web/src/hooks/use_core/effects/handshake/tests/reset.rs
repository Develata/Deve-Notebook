use super::*;

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
    let (pending_branch_switch, set_pending_branch_switch) = signal(None::<PendingBranchSwitch>);
    let (pending_repo_switch, set_pending_repo_switch) = signal(None::<PendingRepoSwitch>);
    let (handshake_scope_nonce, set_handshake_scope_nonce) = signal(Some(7u64));
    let (handshake_retry_nonce, _) = signal(0u64);
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
        pending_repo_switch,
        set_pending_repo_switch,
        handshake_scope_nonce,
        set_handshake_scope_nonce,
        handshake_retry_nonce,
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
