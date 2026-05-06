//! plan_ref:
//!   - 08_ui_design_02_desktop#desktop-native-adapter-contract
//!   - 08_ui_design_03_mobile#mobile-native-adapter-contract
//!   - 09_auth#unauthorized-disconnected-ui
//!

use crate::api::ConnectionStatus;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SyncStatusKind {
    SessionExpired,
    NativeBootstrapInvalid,
    NativeSessionPending,
    NativeServiceOffline,
    NativeReprobeRequired,
    Offline,
    Reconnecting,
    SnapshotLoading,
    ReadOnly,
    HandshakingRepo,
    PendingAck,
    Ready,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct SyncStatusSummary {
    pub kind: SyncStatusKind,
    pub repo_name: Option<String>,
    pub pending_ack_count: usize,
}

impl SyncStatusSummary {
    pub fn header_text(&self) -> &'static str {
        match self.kind {
            SyncStatusKind::SessionExpired => "Session Expired",
            SyncStatusKind::NativeBootstrapInvalid => "Native Bootstrap Invalid",
            SyncStatusKind::NativeSessionPending => "Native Session Pending",
            SyncStatusKind::NativeServiceOffline => "Native Service Offline",
            SyncStatusKind::NativeReprobeRequired => "Native Reprobe Required",
            SyncStatusKind::Offline => "Offline",
            SyncStatusKind::Reconnecting => "Reconnecting",
            SyncStatusKind::SnapshotLoading => "Loading Snapshot",
            SyncStatusKind::ReadOnly => "Read-only",
            SyncStatusKind::HandshakingRepo => "Handshaking repo",
            SyncStatusKind::PendingAck => "Pending Ack",
            SyncStatusKind::Ready => "Ready",
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn derive_sync_status(
    connection_status: ConnectionStatus,
    load_state: &str,
    remote_branch_active: bool,
    degraded_storage: bool,
    node_role_probe_failed: bool,
    node_role_readable: bool,
    handshake_ready: bool,
    writer_ready: bool,
    current_repo_id: Option<&str>,
    current_repo_name: Option<&str>,
    pending_repo_switch: Option<&str>,
    pending_branch_switch: bool,
    pending_ack_count: usize,
) -> SyncStatusSummary {
    let repo_name = pending_repo_switch
        .or(current_repo_name)
        .map(ToOwned::to_owned);
    let kind = match connection_status {
        ConnectionStatus::Unauthorized => SyncStatusKind::SessionExpired,
        ConnectionStatus::NativeBootstrapInvalid => SyncStatusKind::NativeBootstrapInvalid,
        ConnectionStatus::NativeSessionPending => SyncStatusKind::NativeSessionPending,
        ConnectionStatus::NativeServiceOffline => SyncStatusKind::NativeServiceOffline,
        ConnectionStatus::NativeReprobeRequired => SyncStatusKind::NativeReprobeRequired,
        ConnectionStatus::Disconnected => SyncStatusKind::Offline,
        ConnectionStatus::Connecting => SyncStatusKind::Reconnecting,
        ConnectionStatus::Connected if node_role_probe_failed => {
            SyncStatusKind::NativeReprobeRequired
        }
        ConnectionStatus::Connected if load_state != "ready" => SyncStatusKind::SnapshotLoading,
        ConnectionStatus::Connected
            if pending_repo_switch.is_some()
                || pending_branch_switch
                || current_repo_id.is_none()
                || !node_role_readable =>
        {
            SyncStatusKind::HandshakingRepo
        }
        ConnectionStatus::Connected if remote_branch_active || degraded_storage => {
            SyncStatusKind::ReadOnly
        }
        ConnectionStatus::Connected if !handshake_ready || !writer_ready => {
            SyncStatusKind::HandshakingRepo
        }
        ConnectionStatus::Connected if pending_ack_count > 0 => SyncStatusKind::PendingAck,
        ConnectionStatus::Connected => SyncStatusKind::Ready,
    };

    SyncStatusSummary {
        kind,
        repo_name,
        pending_ack_count,
    }
}

#[cfg(test)]
#[path = "status_summary_tests.rs"]
mod tests;
