use super::{
    SnapshotRequestMatch, confirmed_history, initial_snapshot_may_auto_reopen,
    reconstruct_full_snapshot_content, snapshot_request_matches,
};
use crate::runtime::domain::PendingBranchTarget;
use deve_core::models::{Op, PeerId};
use deve_core::protocol::ConfirmedOp;

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

#[test]
fn snapshot_delta_fallback_borrows_confirmed_ops() {
    let ops = vec![
        ConfirmedOp::new(
            3,
            Op::Insert {
                pos: 3,
                content: "X".into(),
            },
            None,
        ),
        ConfirmedOp::new(4, Op::Delete { pos: 1, len: 2 }, None),
    ];

    assert_eq!(
        reconstruct_full_snapshot_content("A😀B", &ops).as_deref(),
        Some("AXB")
    );
    assert_eq!(confirmed_history(&ops).len(), 2);
}

#[test]
fn initial_snapshot_adapter_failure_auto_reopens_at_most_once() {
    assert!(initial_snapshot_may_auto_reopen(false));
    assert!(!initial_snapshot_may_auto_reopen(true));
}
