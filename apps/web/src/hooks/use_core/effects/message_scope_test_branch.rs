use super::*;

#[test]
fn peer_scope_prefers_pending_branch_target() {
    assert!(!peer_branch_matches_scope(
        &None,
        None,
        Some(PendingBranchTarget::Shadow("peer-a".into())),
    ));
    assert!(peer_branch_matches_scope(
        &Some(PeerId::new("peer-a")),
        None,
        Some(PendingBranchTarget::Shadow("peer-a".into())),
    ));
}

#[test]
fn string_scope_accepts_pending_local_branch() {
    assert!(string_branch_matches_scope(
        &None,
        Some(PeerId::new("peer-a")),
        Some(PendingBranchTarget::Local),
    ));
    assert!(!string_branch_matches_scope(
        &Some("peer-a".into()),
        Some(PeerId::new("peer-a")),
        Some(PendingBranchTarget::Local),
    ));
}
