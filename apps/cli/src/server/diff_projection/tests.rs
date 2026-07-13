use super::{DiffJobGate, DiffProjectionExecutor};
use crate::server::channel::DualChannel;
use deve_core::models::{PeerId, RepoId};
use deve_core::protocol::{ScopeNonce, ServerError, ServerErrorCode, ServerMessage};
use std::sync::Arc;
use std::sync::atomic::Ordering;
use tokio::sync::{broadcast, mpsc};

#[test]
fn new_revision_cancels_previous_diff_job() {
    let gate = DiffJobGate::new();
    let repo_id = RepoId::new_v4();
    let first = gate
        .begin_draft("first".into(), 1, repo_id, None, ScopeNonce::new(7))
        .unwrap();
    let _second = gate.begin_draft(
        "second".into(),
        2,
        repo_id,
        Some(PeerId::new("peer")),
        ScopeNonce::new(7),
    );
    assert!(first.cancellation.cancelled.load(Ordering::Acquire));
    assert!(
        gate.begin_draft("stale".into(), 1, repo_id, None, ScopeNonce::new(7))
            .is_none()
    );
    gate.cancel();
    assert!(
        gate.begin_draft("new-scope".into(), 1, repo_id, None, ScopeNonce::new(8))
            .is_some()
    );
}

#[tokio::test]
async fn loaded_diff_failure_keeps_request_and_scope_identity() {
    let gate = DiffJobGate::new();
    let repo_id = RepoId::new_v4();
    let branch = Some(PeerId::new("peer-a"));
    let ticket = gate.begin_fixed(
        "commit-file-1".into(),
        repo_id,
        branch.clone(),
        ScopeNonce::new(9),
    );
    let (broadcast_tx, _) = broadcast::channel(2);
    let (unicast_tx, mut unicast_rx) = mpsc::channel(2);
    let channel = DualChannel::new(broadcast_tx, unicast_tx);
    Arc::new(DiffProjectionExecutor::new()).spawn_loaded(
        ticket,
        || {
            Err(ServerError::with_detail(
                ServerErrorCode::DiffComputeFailed,
                "commit target mismatch",
            ))
        },
        channel,
    );
    match tokio::time::timeout(std::time::Duration::from_secs(2), unicast_rx.recv())
        .await
        .expect("diff error timeout")
    {
        Some(ServerMessage::DiffProjectionError {
            request_id,
            revision,
            repo_id: actual_repo,
            branch: actual_branch,
            scope_nonce,
            error,
        }) => {
            assert_eq!(request_id, "commit-file-1");
            assert_eq!(revision, 0);
            assert_eq!(actual_repo, repo_id);
            assert_eq!(actual_branch, branch);
            assert_eq!(scope_nonce, ScopeNonce::new(9));
            assert_eq!(error.code, ServerErrorCode::DiffComputeFailed);
        }
        other => panic!("expected correlated diff error, got {other:?}"),
    }
}
