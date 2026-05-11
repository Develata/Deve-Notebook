use super::*;

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
    let local = handshake_mode_key("ws://a", None, Some("repo-1"), None, 7);
    let shadow = handshake_mode_key(
        "ws://a",
        None,
        Some("repo-1"),
        Some(&PeerId::new("peer-a")),
        7,
    );
    assert_ne!(local, shadow);
}

#[test]
fn handshake_mode_key_distinguishes_scope_nonce_generations() {
    let old_scope = handshake_mode_key("ws://a", None, Some("repo-1"), None, 7);
    let new_scope = handshake_mode_key("ws://a", None, Some("repo-1"), None, 8);
    assert_ne!(old_scope, new_scope);
}

#[test]
fn suspended_mode_key_does_not_collide_with_active_scope_keys() {
    let local = handshake_mode_key("ws://a", None, Some("repo-1"), None, 7)
        .expect("local scope should have handshake mode key");
    let shadow = handshake_mode_key(
        "ws://a",
        None,
        Some("repo-1"),
        Some(&PeerId::new("peer-a")),
        7,
    )
    .expect("shadow scope should have handshake mode key");
    let suspended = suspended_handshake_mode_key("ws://a");
    assert_ne!(suspended, local);
    assert_ne!(suspended, shadow);
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
