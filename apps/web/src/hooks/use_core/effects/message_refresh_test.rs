use super::{RefreshScope, capture_refresh_scope, should_send_refresh};
use crate::hooks::use_core::PendingBranchTarget;
use deve_core::models::PeerId;

#[test]
fn does_not_capture_refresh_scope_during_switch() {
    assert_eq!(
        capture_refresh_scope(
            Some("repo-a".into()),
            None,
            Some(PendingBranchTarget::Local),
            None,
            3,
        ),
        None,
    );
}

#[test]
fn does_not_capture_refresh_scope_for_remote_branch() {
    assert_eq!(
        capture_refresh_scope(
            Some("repo-a".into()),
            Some(PeerId::new("peer-a")),
            None,
            None,
            4,
        ),
        None,
    );
}

#[test]
fn rejects_refresh_after_repo_scope_changes() {
    let scope = RefreshScope {
        repo_id: Some("repo-a".into()),
        branch: Some(PeerId::new("peer-a")),
        scope_nonce: 3,
    };
    assert!(!should_send_refresh(
        &scope,
        Some("repo-b".into()),
        Some(PeerId::new("peer-a")),
        None,
        None,
        3,
    ));
    assert!(!should_send_refresh(
        &scope,
        Some("repo-a".into()),
        Some(PeerId::new("peer-b")),
        None,
        None,
        3,
    ));
    assert!(!should_send_refresh(
        &scope,
        Some("repo-a".into()),
        Some(PeerId::new("peer-a")),
        None,
        None,
        4,
    ));
}

#[test]
fn keeps_refresh_only_when_scope_is_unchanged() {
    let scope = RefreshScope {
        repo_id: Some("repo-a".into()),
        branch: None,
        scope_nonce: 5,
    };
    assert!(should_send_refresh(
        &scope,
        Some("repo-a".into()),
        None,
        None,
        None,
        5,
    ));
}
