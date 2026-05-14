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
    PeerNotRegistered,
    PendingAck,
    Ready,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct SyncStatusSummary {
    pub kind: SyncStatusKind,
    pub repo_name: Option<String>,
    pub pending_ack_count: usize,
}

pub(crate) struct SyncStatusInput<'a> {
    pub connection_status: ConnectionStatus,
    pub load_state: &'a str,
    pub remote_branch_active: bool,
    pub degraded_storage: bool,
    pub node_role_probe_failed: bool,
    pub node_role_readable: bool,
    pub handshake_ready: bool,
    pub writer_ready: bool,
    pub current_repo_id: Option<&'a str>,
    pub current_repo_name: Option<&'a str>,
    pub pending_repo_switch: Option<&'a str>,
    pub pending_branch_switch: bool,
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
            SyncStatusKind::PeerNotRegistered => "Logged in / Peer not registered",
            SyncStatusKind::PendingAck => "Pending Ack",
            SyncStatusKind::Ready => "Ready",
        }
    }
}

pub(crate) fn derive_sync_status(input: SyncStatusInput<'_>) -> SyncStatusSummary {
    let repo_name = input
        .pending_repo_switch
        .or(input.current_repo_name)
        .map(ToOwned::to_owned);
    let kind = match input.connection_status {
        ConnectionStatus::Unauthorized => SyncStatusKind::SessionExpired,
        ConnectionStatus::NativeBootstrapInvalid => SyncStatusKind::NativeBootstrapInvalid,
        ConnectionStatus::NativeSessionPending => SyncStatusKind::NativeSessionPending,
        ConnectionStatus::NativeServiceOffline => SyncStatusKind::NativeServiceOffline,
        ConnectionStatus::NativeReprobeRequired => SyncStatusKind::NativeReprobeRequired,
        ConnectionStatus::Disconnected => SyncStatusKind::Offline,
        ConnectionStatus::Connecting => SyncStatusKind::Reconnecting,
        ConnectionStatus::Connected if input.node_role_probe_failed => {
            SyncStatusKind::NativeReprobeRequired
        }
        ConnectionStatus::Connected if input.load_state != "ready" => {
            SyncStatusKind::SnapshotLoading
        }
        ConnectionStatus::Connected
            if input.pending_repo_switch.is_some()
                || input.pending_branch_switch
                || input.current_repo_id.is_none()
                || !input.node_role_readable =>
        {
            SyncStatusKind::HandshakingRepo
        }
        ConnectionStatus::Connected if input.remote_branch_active || input.degraded_storage => {
            SyncStatusKind::ReadOnly
        }
        ConnectionStatus::Connected if !input.handshake_ready || !input.writer_ready => {
            SyncStatusKind::PeerNotRegistered
        }
        ConnectionStatus::Connected if input.pending_ack_count > 0 => SyncStatusKind::PendingAck,
        ConnectionStatus::Connected => SyncStatusKind::Ready,
    };

    SyncStatusSummary {
        kind,
        repo_name,
        pending_ack_count: input.pending_ack_count,
    }
}

#[cfg(test)]
mod tests;
