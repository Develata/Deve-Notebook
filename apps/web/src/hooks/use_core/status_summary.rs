//! plan_ref:
//!   - 11_ui_design/02_desktop#desktop-native-adapter-contract
//!   - 11_ui_design/03_mobile#mobile-native-adapter-contract
//!   - 08_auth#unauthorized-disconnected-ui
//!   - 13_i18n#i18n-facade-contract
//!

use crate::api::ConnectionStatus;
use crate::i18n::{Locale, t};

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
    pub fn display_text(&self, locale: Locale) -> String {
        match self.kind {
            SyncStatusKind::SessionExpired => t::bottom_bar::unauthorized(locale).to_string(),
            SyncStatusKind::NativeBootstrapInvalid => {
                t::bottom_bar::native_bootstrap_invalid(locale).to_string()
            }
            SyncStatusKind::NativeSessionPending => {
                t::bottom_bar::native_session_pending(locale).to_string()
            }
            SyncStatusKind::NativeServiceOffline => {
                t::bottom_bar::native_service_offline(locale).to_string()
            }
            SyncStatusKind::NativeReprobeRequired => {
                t::bottom_bar::native_reprobe_required(locale).to_string()
            }
            SyncStatusKind::Offline => t::bottom_bar::offline(locale).to_string(),
            SyncStatusKind::Reconnecting => t::bottom_bar::reconnecting(locale).to_string(),
            SyncStatusKind::SnapshotLoading => t::bottom_bar::snapshot_loading(locale).to_string(),
            SyncStatusKind::ReadOnly => t::bottom_bar::read_only(locale).to_string(),
            SyncStatusKind::HandshakingRepo => t::bottom_bar::handshaking_repo(locale).to_string(),
            SyncStatusKind::PeerNotRegistered => {
                t::bottom_bar::peer_not_registered(locale).to_string()
            }
            SyncStatusKind::PendingAck => {
                t::bottom_bar::pending_ack(locale, self.pending_ack_count)
            }
            SyncStatusKind::Ready => t::bottom_bar::ready(locale).to_string(),
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
