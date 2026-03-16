use super::{
    ScopedMessageScope, SyncPayloadScope, accepts_sync_payload, matches_scope,
    matches_scoped_message,
};
use crate::hooks::use_core::PendingBranchTarget;
use deve_core::models::PeerId;

#[test]
fn matches_scope_rejects_same_repo_on_different_branch() {
    let repo_id = uuid::Uuid::new_v4();
    assert!(!matches_scope(
        Some(repo_id.to_string()),
        None,
        Some(PeerId::new("peer-b")),
        None,
        Some(repo_id),
        Some(PeerId::new("peer-a")),
    ));
}

#[test]
fn matches_scope_accepts_same_repo_and_branch() {
    let repo_id = uuid::Uuid::new_v4();
    assert!(matches_scope(
        Some(repo_id.to_string()),
        None,
        Some(PeerId::new("peer-a")),
        None,
        Some(repo_id),
        Some(PeerId::new("peer-a")),
    ));
}

#[test]
fn matches_scope_rejects_same_repo_without_branch_when_remote_active() {
    let repo_id = uuid::Uuid::new_v4();
    assert!(!matches_scope(
        Some(repo_id.to_string()),
        None,
        Some(PeerId::new("peer-a")),
        None,
        Some(repo_id),
        None,
    ));
}

#[test]
fn matches_scope_rejects_repo_less_message_once_repo_is_bound() {
    assert!(!matches_scope(
        Some(uuid::Uuid::new_v4().to_string()),
        None,
        None,
        None,
        None,
        None,
    ));
}

#[test]
fn matches_scope_accepts_repo_less_message_before_repo_binding() {
    assert!(matches_scope(None, None, None, None, None, None));
}

#[test]
fn matches_scope_rejects_messages_while_repo_switch_pending() {
    let repo_id = uuid::Uuid::new_v4();
    assert!(!matches_scope(
        Some(repo_id.to_string()),
        Some("test".into()),
        None,
        None,
        Some(repo_id),
        None,
    ));
}

#[test]
fn matches_scope_rejects_messages_while_branch_switch_pending() {
    let repo_id = uuid::Uuid::new_v4();
    assert!(!matches_scope(
        Some(repo_id.to_string()),
        None,
        Some(PeerId::new("peer-a")),
        Some(PendingBranchTarget::Local),
        Some(repo_id),
        None,
    ));
}

#[test]
fn matches_scope_prefers_pending_branch_target() {
    let repo_id = uuid::Uuid::new_v4();
    assert!(!matches_scope(
        Some(repo_id.to_string()),
        None,
        Some(PeerId::new("peer-a")),
        Some(PendingBranchTarget::Shadow("peer-b".into())),
        Some(repo_id),
        Some(PeerId::new("peer-b")),
    ));
}

#[test]
fn accepts_sync_payload_only_for_current_handshake_scope() {
    let repo_id = uuid::Uuid::new_v4();
    assert!(accepts_sync_payload(
        SyncPayloadScope {
            current_repo_id: Some(repo_id.to_string()),
            pending_repo_switch: None,
            current_branch: None,
            pending_branch_switch: None,
            handshake_scope_nonce: Some(5),
        },
        repo_id,
        None,
        5,
    ));
    assert!(!accepts_sync_payload(
        SyncPayloadScope {
            current_repo_id: Some(repo_id.to_string()),
            pending_repo_switch: None,
            current_branch: None,
            pending_branch_switch: None,
            handshake_scope_nonce: Some(6),
        },
        repo_id,
        None,
        5,
    ));
}

#[test]
fn rejects_sync_payload_while_scope_switch_is_pending() {
    let repo_id = uuid::Uuid::new_v4();
    assert!(!accepts_sync_payload(
        SyncPayloadScope {
            current_repo_id: Some(repo_id.to_string()),
            pending_repo_switch: Some("test".into()),
            current_branch: None,
            pending_branch_switch: None,
            handshake_scope_nonce: Some(5),
        },
        repo_id,
        None,
        5,
    ));
}

#[test]
fn matches_scoped_message_accepts_matching_nonce_and_scope() {
    let repo_id = uuid::Uuid::new_v4();
    assert!(matches_scoped_message(
        ScopedMessageScope {
            current_repo_id: Some(repo_id.to_string()),
            pending_repo_switch: None,
            current_branch: Some(PeerId::new("peer-a")),
            pending_branch_switch: None,
            current_scope_nonce: 7,
        },
        Some(repo_id),
        Some(PeerId::new("peer-a")),
        Some(7),
    ));
}

#[test]
fn matches_scoped_message_rejects_stale_nonce() {
    let repo_id = uuid::Uuid::new_v4();
    assert!(!matches_scoped_message(
        ScopedMessageScope {
            current_repo_id: Some(repo_id.to_string()),
            pending_repo_switch: None,
            current_branch: None,
            pending_branch_switch: None,
            current_scope_nonce: 7,
        },
        Some(repo_id),
        None,
        Some(6),
    ));
}
