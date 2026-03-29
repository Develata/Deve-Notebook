use super::*;

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
