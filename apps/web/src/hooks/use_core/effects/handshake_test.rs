use super::{handshake_mode_key, should_restore_session_scope, should_suspend_handshake};
use crate::hooks::use_core::PendingBranchTarget;
use deve_core::models::PeerId;

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
