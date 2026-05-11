use super::*;

#[test]
fn shadow_list_rejects_messages_while_switch_pending() {
    assert!(!shadow_list_matches_scope(
        RequestMatch {
            message_id: None,
            expected_id: None,
            scope_nonce: Some(3),
            current_scope_nonce: 3,
        },
        &ShadowListScope {
            pending_branch_switch: Some(PendingBranchTarget::Shadow("peer-a".into())),
            pending_repo_switch: None,
        },
    ));
    assert!(!shadow_list_matches_scope(
        RequestMatch {
            message_id: None,
            expected_id: None,
            scope_nonce: Some(3),
            current_scope_nonce: 3,
        },
        &ShadowListScope {
            pending_branch_switch: None,
            pending_repo_switch: Some("default".into()),
        },
    ));
    assert!(shadow_list_matches_scope(
        RequestMatch {
            message_id: None,
            expected_id: None,
            scope_nonce: Some(3),
            current_scope_nonce: 3,
        },
        &ShadowListScope {
            pending_branch_switch: None,
            pending_repo_switch: None,
        },
    ));
}

#[test]
fn scoped_list_accepts_matching_request_id_only() {
    assert!(!shadow_list_matches_scope(
        RequestMatch {
            message_id: Some("req-1"),
            expected_id: Some("req-1"),
            scope_nonce: Some(7),
            current_scope_nonce: 3,
        },
        &ShadowListScope {
            pending_branch_switch: None,
            pending_repo_switch: None,
        },
    ));
    assert!(shadow_list_matches_scope(
        RequestMatch {
            message_id: Some("req-1"),
            expected_id: Some("req-1"),
            scope_nonce: Some(3),
            current_scope_nonce: 3,
        },
        &ShadowListScope {
            pending_branch_switch: None,
            pending_repo_switch: None,
        },
    ));
    assert!(!shadow_list_matches_scope(
        RequestMatch {
            message_id: Some("stale"),
            expected_id: Some("req-1"),
            scope_nonce: Some(7),
            current_scope_nonce: 3,
        },
        &ShadowListScope {
            pending_branch_switch: None,
            pending_repo_switch: None,
        },
    ));
}
