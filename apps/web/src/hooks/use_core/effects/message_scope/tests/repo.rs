use super::*;

#[test]
fn repo_list_rejects_messages_while_branch_switch_pending() {
    assert!(!repo_list_matches_scope(
        RequestMatch {
            message_id: None,
            expected_id: None,
            scope_nonce: Some(3),
            current_scope_nonce: 3,
        },
        None,
        &RepoListScope {
            active_branch: None,
            pending_branch_switch: Some(PendingBranchTarget::Shadow("peer-a".into())),
            pending_repo_switch: None,
        },
    ));
}

#[test]
fn repo_list_uses_active_branch_without_pending_switch() {
    assert!(repo_list_matches_scope(
        RequestMatch {
            message_id: None,
            expected_id: None,
            scope_nonce: Some(3),
            current_scope_nonce: 3,
        },
        Some("peer-a".into()),
        &RepoListScope {
            active_branch: Some(PeerId::new("peer-a")),
            pending_branch_switch: None,
            pending_repo_switch: None,
        },
    ));
    assert!(!repo_list_matches_scope(
        RequestMatch {
            message_id: None,
            expected_id: None,
            scope_nonce: Some(3),
            current_scope_nonce: 3,
        },
        Some("peer-b".into()),
        &RepoListScope {
            active_branch: Some(PeerId::new("peer-a")),
            pending_branch_switch: None,
            pending_repo_switch: None,
        },
    ));
}

#[test]
fn repo_list_rejects_messages_while_repo_switch_pending() {
    assert!(!repo_list_matches_scope(
        RequestMatch {
            message_id: None,
            expected_id: None,
            scope_nonce: Some(3),
            current_scope_nonce: 3,
        },
        None,
        &RepoListScope {
            active_branch: None,
            pending_branch_switch: None,
            pending_repo_switch: Some("default".into()),
        },
    ));
}

#[test]
fn repo_list_rejects_stale_system_scope_nonce() {
    assert!(!repo_list_matches_scope(
        RequestMatch {
            message_id: None,
            expected_id: None,
            scope_nonce: Some(2),
            current_scope_nonce: 3,
        },
        None,
        &RepoListScope {
            active_branch: None,
            pending_branch_switch: None,
            pending_repo_switch: None,
        },
    ));
}
