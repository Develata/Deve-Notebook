use super::{
    RepoListScope, RequestMatch, ShadowListScope, accepts_system_or_matching_request,
    peer_branch_matches_scope, repo_list_matches_scope, shadow_list_matches_scope,
    string_branch_matches_scope,
};
use crate::hooks::use_core::PendingBranchTarget;
use deve_core::models::PeerId;

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
    assert!(shadow_list_matches_scope(
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

#[test]
fn system_or_matching_request_accepts_none_and_exact_match() {
    assert!(accepts_system_or_matching_request(None, None, Some(3), 3));
    assert!(!accepts_system_or_matching_request(
        None,
        Some("req-1"),
        Some(3),
        3,
    ));
    assert!(accepts_system_or_matching_request(
        Some("req-1"),
        Some("req-1"),
        Some(7),
        3,
    ));
    assert!(!accepts_system_or_matching_request(
        Some("stale"),
        Some("req-1"),
        Some(7),
        3,
    ));
    assert!(!accepts_system_or_matching_request(None, None, Some(2), 3));
    assert!(!accepts_system_or_matching_request(None, None, None, 3));
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
