use super::{SnapshotRequestMatch, snapshot_request_matches};
use crate::hooks::use_core::PendingBranchTarget;
use deve_core::models::PeerId;

#[test]
fn snapshot_request_rejects_pending_repo_switch() {
    let repo_id = uuid::Uuid::new_v4();
    assert!(!snapshot_request_matches(SnapshotRequestMatch {
        open_request_id: 7,
        request_id: 7,
        current_repo_id: Some(repo_id.to_string()),
        pending_repo_switch: Some("test".into()),
        active_branch: None,
        pending_branch_switch: None,
        current_scope_nonce: 7,
        scope_nonce: 7,
        current_generation: 7,
        expected_generation: 7,
        repo_id,
        branch: None,
    }));
}

#[test]
fn snapshot_request_rejects_branch_mismatch_even_with_same_request_id() {
    let repo_id = uuid::Uuid::new_v4();
    assert!(!snapshot_request_matches(SnapshotRequestMatch {
        open_request_id: 7,
        request_id: 7,
        current_repo_id: Some(repo_id.to_string()),
        pending_repo_switch: None,
        active_branch: Some(PeerId::new("peer-a")),
        pending_branch_switch: None,
        current_scope_nonce: 7,
        scope_nonce: 7,
        current_generation: 7,
        expected_generation: 7,
        repo_id,
        branch: Some(PeerId::new("peer-b")),
    }));
    assert!(!snapshot_request_matches(SnapshotRequestMatch {
        open_request_id: 7,
        request_id: 7,
        current_repo_id: Some(repo_id.to_string()),
        pending_repo_switch: None,
        active_branch: Some(PeerId::new("peer-a")),
        pending_branch_switch: Some(PendingBranchTarget::Local),
        current_scope_nonce: 7,
        scope_nonce: 7,
        current_generation: 7,
        expected_generation: 7,
        repo_id,
        branch: None,
    }));
    assert!(snapshot_request_matches(SnapshotRequestMatch {
        open_request_id: 7,
        request_id: 7,
        current_repo_id: Some(repo_id.to_string()),
        pending_repo_switch: None,
        active_branch: None,
        pending_branch_switch: None,
        current_scope_nonce: 7,
        scope_nonce: 7,
        current_generation: 7,
        expected_generation: 7,
        repo_id,
        branch: None,
    }));
}

#[test]
fn snapshot_request_rejects_scope_nonce_mismatch_even_with_same_repo_and_request_id() {
    let repo_id = uuid::Uuid::new_v4();
    assert!(!snapshot_request_matches(SnapshotRequestMatch {
        open_request_id: 7,
        request_id: 7,
        current_repo_id: Some(repo_id.to_string()),
        pending_repo_switch: None,
        active_branch: None,
        pending_branch_switch: None,
        current_scope_nonce: 9,
        scope_nonce: 7,
        current_generation: 7,
        expected_generation: 7,
        repo_id,
        branch: None,
    }));
}
