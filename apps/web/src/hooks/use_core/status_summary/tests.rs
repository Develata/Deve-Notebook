//! plan_ref:
//!   - 07_network#web-ws-runtime
//!
use super::{SyncStatusInput, SyncStatusKind, SyncStatusSummary, derive_sync_status};
use crate::api::ConnectionStatus;
use crate::i18n::Locale;

fn ready_input() -> SyncStatusInput<'static> {
    SyncStatusInput {
        connection_status: ConnectionStatus::Connected,
        load_state: "ready",
        remote_branch_active: false,
        degraded_storage: false,
        node_role_probe_failed: false,
        node_role_readable: true,
        handshake_ready: true,
        writer_ready: true,
        current_repo_id: Some("repo-id"),
        current_repo_name: Some("default"),
        pending_repo_switch: None,
        pending_branch_switch: false,
        pending_ack_count: 0,
    }
}

fn summary(input: SyncStatusInput<'static>) -> SyncStatusSummary {
    derive_sync_status(input)
}

#[test]
fn reports_session_expired_for_unauthorized_status() {
    let summary = summary(SyncStatusInput {
        connection_status: ConnectionStatus::Unauthorized,
        ..ready_input()
    });
    assert_eq!(summary.kind, SyncStatusKind::SessionExpired);
    assert_eq!(summary.display_text(Locale::En), "Session Expired");
    assert_eq!(summary.display_text(Locale::Zh), "会话已过期");
}

#[test]
fn reports_native_session_pending_before_snapshot_state() {
    let summary = summary(SyncStatusInput {
        connection_status: ConnectionStatus::NativeSessionPending,
        ..ready_input()
    });
    assert_eq!(summary.kind, SyncStatusKind::NativeSessionPending);
    assert_eq!(summary.display_text(Locale::En), "Native Session Pending");
}

#[test]
fn reports_native_service_offline_as_specific_recovery_state() {
    let summary = summary(SyncStatusInput {
        connection_status: ConnectionStatus::NativeServiceOffline,
        load_state: "loading",
        node_role_readable: false,
        handshake_ready: false,
        writer_ready: false,
        ..ready_input()
    });
    assert_eq!(summary.kind, SyncStatusKind::NativeServiceOffline);
    assert_eq!(summary.display_text(Locale::En), "Native Service Offline");
}

#[test]
fn prefers_loading_state_while_snapshot_is_inflight() {
    let summary = summary(SyncStatusInput {
        load_state: "loading",
        node_role_readable: false,
        handshake_ready: false,
        writer_ready: false,
        ..ready_input()
    });
    assert_eq!(summary.kind, SyncStatusKind::SnapshotLoading);
}

#[test]
fn reports_editor_sync_error_instead_of_permanent_snapshot_loading() {
    let summary = summary(SyncStatusInput {
        load_state: "error",
        ..ready_input()
    });
    assert_eq!(summary.kind, SyncStatusKind::EditorSyncError);
    assert_eq!(summary.display_text(Locale::En), "Editor sync error");
}

#[test]
fn reports_native_reprobe_before_snapshot_loading_when_node_role_probe_failed() {
    let summary = summary(SyncStatusInput {
        load_state: "loading",
        node_role_probe_failed: true,
        node_role_readable: false,
        handshake_ready: false,
        writer_ready: false,
        ..ready_input()
    });
    assert_eq!(summary.kind, SyncStatusKind::NativeReprobeRequired);
}

#[test]
fn reports_read_only_for_remote_branch_views() {
    let summary = summary(SyncStatusInput {
        remote_branch_active: true,
        handshake_ready: false,
        writer_ready: false,
        ..ready_input()
    });
    assert_eq!(summary.kind, SyncStatusKind::ReadOnly);
}

#[test]
fn reports_peer_not_registered_until_writer_is_ready() {
    let summary = summary(SyncStatusInput {
        writer_ready: false,
        ..ready_input()
    });
    assert_eq!(summary.kind, SyncStatusKind::PeerNotRegistered);
    assert_eq!(
        summary.display_text(Locale::En),
        "Logged in / Peer not registered"
    );
}

#[test]
fn reports_repo_handshake_until_node_role_is_readable() {
    let summary = summary(SyncStatusInput {
        node_role_readable: false,
        ..ready_input()
    });
    assert_eq!(summary.kind, SyncStatusKind::HandshakingRepo);
}

#[test]
fn reports_native_reprobe_when_node_role_probe_failed() {
    let summary = summary(SyncStatusInput {
        node_role_probe_failed: true,
        node_role_readable: false,
        ..ready_input()
    });
    assert_eq!(summary.kind, SyncStatusKind::NativeReprobeRequired);
}

#[test]
fn reports_repo_handshake_for_remote_branch_until_node_role_is_readable() {
    let summary = summary(SyncStatusInput {
        remote_branch_active: true,
        node_role_readable: false,
        ..ready_input()
    });
    assert_eq!(summary.kind, SyncStatusKind::HandshakingRepo);
}

#[test]
fn reports_handshaking_repo_while_repo_switch_is_pending() {
    let summary = summary(SyncStatusInput {
        pending_repo_switch: Some("other"),
        writer_ready: false,
        ..ready_input()
    });
    assert_eq!(summary.kind, SyncStatusKind::HandshakingRepo);
}

#[test]
fn reports_pending_ack_after_handshake_is_confirmed() {
    let summary = summary(SyncStatusInput {
        pending_ack_count: 3,
        ..ready_input()
    });
    assert_eq!(summary.kind, SyncStatusKind::PendingAck);
    assert_eq!(summary.pending_ack_count, 3);
    assert_eq!(summary.display_text(Locale::Zh), "等待确认 (3)");
}

#[test]
fn localizes_ready_status_for_header_and_status_surfaces() {
    let summary = summary(ready_input());

    assert_eq!(summary.kind, SyncStatusKind::Ready);
    assert_eq!(summary.kind.marker(), "ready");
    assert_eq!(summary.display_text(Locale::En), "Ready");
    assert_eq!(summary.display_text(Locale::Zh), "就绪");
}
