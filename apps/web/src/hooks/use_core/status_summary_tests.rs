//! plan_ref:
//!   - 05_network#web-ws-runtime
//!
use super::{SyncStatusKind, derive_sync_status};
use crate::api::ConnectionStatus;

#[test]
fn reports_session_expired_for_unauthorized_status() {
    let summary = derive_sync_status(
        ConnectionStatus::Unauthorized,
        "ready",
        false,
        false,
        false,
        true,
        true,
        true,
        Some("repo-id"),
        Some("default"),
        None,
        false,
        0,
    );
    assert_eq!(summary.kind, SyncStatusKind::SessionExpired);
    assert_eq!(summary.header_text(), "Session Expired");
}

#[test]
fn reports_native_session_pending_before_snapshot_state() {
    let summary = derive_sync_status(
        ConnectionStatus::NativeSessionPending,
        "ready",
        false,
        false,
        false,
        true,
        true,
        true,
        Some("repo-id"),
        Some("default"),
        None,
        false,
        0,
    );
    assert_eq!(summary.kind, SyncStatusKind::NativeSessionPending);
    assert_eq!(summary.header_text(), "Native Session Pending");
}

#[test]
fn reports_native_service_offline_as_specific_recovery_state() {
    let summary = derive_sync_status(
        ConnectionStatus::NativeServiceOffline,
        "loading",
        false,
        false,
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
    assert_eq!(summary.kind, SyncStatusKind::NativeServiceOffline);
    assert_eq!(summary.header_text(), "Native Service Offline");
}

#[test]
fn prefers_loading_state_while_snapshot_is_inflight() {
    let summary = derive_sync_status(
        ConnectionStatus::Connected,
        "loading",
        false,
        false,
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
        true,
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
        false,
        true,
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
fn reports_repo_handshake_until_node_role_is_readable() {
    let summary = derive_sync_status(
        ConnectionStatus::Connected,
        "ready",
        false,
        false,
        false,
        false,
        true,
        true,
        Some("repo-id"),
        Some("default"),
        None,
        false,
        0,
    );
    assert_eq!(summary.kind, SyncStatusKind::HandshakingRepo);
}

#[test]
fn reports_native_reprobe_when_node_role_probe_failed() {
    let summary = derive_sync_status(
        ConnectionStatus::Connected,
        "ready",
        false,
        false,
        true,
        false,
        true,
        true,
        Some("repo-id"),
        Some("default"),
        None,
        false,
        0,
    );
    assert_eq!(summary.kind, SyncStatusKind::NativeReprobeRequired);
}

#[test]
fn reports_repo_handshake_for_remote_branch_until_node_role_is_readable() {
    let summary = derive_sync_status(
        ConnectionStatus::Connected,
        "ready",
        true,
        false,
        false,
        false,
        true,
        true,
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
        false,
        true,
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
