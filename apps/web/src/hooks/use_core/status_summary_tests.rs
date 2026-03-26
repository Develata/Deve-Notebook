use super::{SyncStatusKind, derive_sync_status};
use crate::api::ConnectionStatus;

#[test]
fn prefers_loading_state_while_snapshot_is_inflight() {
    let summary = derive_sync_status(
        ConnectionStatus::Connected,
        "loading",
        false,
        false,
        false,
        false,
        Some("repo-id"),
        Some("default"),
        None,
        false,
        0,
    );
    assert_eq!(summary.kind, SyncStatusKind::SnapshotLoading);
}

#[test]
fn reports_read_only_for_remote_branch_views() {
    let summary = derive_sync_status(
        ConnectionStatus::Connected,
        "ready",
        true,
        false,
        false,
        false,
        Some("repo-id"),
        Some("default"),
        None,
        false,
        0,
    );
    assert_eq!(summary.kind, SyncStatusKind::ReadOnly);
}

#[test]
fn reports_repo_handshake_until_writer_is_ready() {
    let summary = derive_sync_status(
        ConnectionStatus::Connected,
        "ready",
        false,
        false,
        true,
        false,
        Some("repo-id"),
        Some("default"),
        None,
        false,
        0,
    );
    assert_eq!(summary.kind, SyncStatusKind::HandshakingRepo);
}

#[test]
fn reports_pending_ack_after_handshake_is_confirmed() {
    let summary = derive_sync_status(
        ConnectionStatus::Connected,
        "ready",
        false,
        false,
        true,
        true,
        Some("repo-id"),
        Some("default"),
        None,
        false,
        3,
    );
    assert_eq!(summary.kind, SyncStatusKind::PendingAck);
    assert_eq!(summary.pending_ack_count, 3);
}
