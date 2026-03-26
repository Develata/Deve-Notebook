use crate::api::ConnectionStatus;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SyncStatusKind {
    SessionExpired,
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
        ConnectionStatus::Disconnected => SyncStatusKind::Offline,
        ConnectionStatus::Connecting => SyncStatusKind::Reconnecting,
        ConnectionStatus::Connected if load_state != "ready" => SyncStatusKind::SnapshotLoading,
        ConnectionStatus::Connected if remote_branch_active || degraded_storage => {
            SyncStatusKind::ReadOnly
        }
        ConnectionStatus::Connected
            if pending_repo_switch.is_some()
                || pending_branch_switch
                || current_repo_id.is_none()
                || !handshake_ready
                || !writer_ready =>
        {
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
