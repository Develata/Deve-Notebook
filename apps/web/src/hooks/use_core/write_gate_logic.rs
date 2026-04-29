//! plan_ref:
//!   - 08_ui_design_02_desktop#desktop-native-adapter-contract
//!   - 08_ui_design_03_mobile#mobile-native-adapter-contract
//!   - 09_auth#unauthorized-handling
//!   - 09_auth#unauthorized-disconnected-ui
//!

use crate::api::ConnectionStatus;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum RepoWriteBlock {
    SessionExpired,
    NativeBootstrapInvalid,
    NativeSessionPending,
    NativeServiceOffline,
    NativeReprobeRequired,
    Offline,
    Reconnecting,
    SnapshotLoading,
    ReadOnly,
    ScopeSwitching,
    NoRepo,
    HandshakingRepo,
}

impl RepoWriteBlock {
    pub fn label(self) -> &'static str {
        match self {
            RepoWriteBlock::SessionExpired => "session expired",
            RepoWriteBlock::NativeBootstrapInvalid => "native bootstrap invalid",
            RepoWriteBlock::NativeSessionPending => "native session pending",
            RepoWriteBlock::NativeServiceOffline => "native service offline",
            RepoWriteBlock::NativeReprobeRequired => "native reprobe required",
            RepoWriteBlock::Offline => "offline",
            RepoWriteBlock::Reconnecting => "reconnecting",
            RepoWriteBlock::SnapshotLoading => "snapshot loading",
            RepoWriteBlock::ReadOnly => "read-only",
            RepoWriteBlock::ScopeSwitching => "scope switching",
            RepoWriteBlock::NoRepo => "no repo selected",
            RepoWriteBlock::HandshakingRepo => "repo handshaking",
        }
    }
}

#[derive(Clone, Copy)]
pub(crate) struct RepoWriteGateState<'a> {
    pub connection_status: ConnectionStatus,
    pub load_state: &'a str,
    pub is_read_only: bool,
    pub handshake_ready: bool,
    pub writer_ready: bool,
    pub has_repo: bool,
    pub pending_branch_switch: bool,
    pub pending_repo_switch: bool,
}

pub(crate) fn repo_write_block(state: RepoWriteGateState<'_>) -> Option<RepoWriteBlock> {
    match state.connection_status {
        ConnectionStatus::Unauthorized => Some(RepoWriteBlock::SessionExpired),
        ConnectionStatus::NativeBootstrapInvalid => Some(RepoWriteBlock::NativeBootstrapInvalid),
        ConnectionStatus::NativeSessionPending => Some(RepoWriteBlock::NativeSessionPending),
        ConnectionStatus::NativeServiceOffline => Some(RepoWriteBlock::NativeServiceOffline),
        ConnectionStatus::NativeReprobeRequired => Some(RepoWriteBlock::NativeReprobeRequired),
        ConnectionStatus::Disconnected => Some(RepoWriteBlock::Offline),
        ConnectionStatus::Connecting => Some(RepoWriteBlock::Reconnecting),
        ConnectionStatus::Connected if state.load_state != "ready" => {
            Some(RepoWriteBlock::SnapshotLoading)
        }
        ConnectionStatus::Connected if state.is_read_only => Some(RepoWriteBlock::ReadOnly),
        ConnectionStatus::Connected if state.pending_branch_switch || state.pending_repo_switch => {
            Some(RepoWriteBlock::ScopeSwitching)
        }
        ConnectionStatus::Connected if !state.has_repo => Some(RepoWriteBlock::NoRepo),
        ConnectionStatus::Connected if !state.handshake_ready || !state.writer_ready => {
            Some(RepoWriteBlock::HandshakingRepo)
        }
        ConnectionStatus::Connected => None,
    }
}

pub(crate) fn repo_source_control_read_block(
    state: RepoWriteGateState<'_>,
) -> Option<RepoWriteBlock> {
    if state.is_read_only {
        return match state.connection_status {
            ConnectionStatus::Unauthorized => Some(RepoWriteBlock::SessionExpired),
            ConnectionStatus::NativeBootstrapInvalid => {
                Some(RepoWriteBlock::NativeBootstrapInvalid)
            }
            ConnectionStatus::NativeSessionPending => Some(RepoWriteBlock::NativeSessionPending),
            ConnectionStatus::NativeServiceOffline => Some(RepoWriteBlock::NativeServiceOffline),
            ConnectionStatus::NativeReprobeRequired => Some(RepoWriteBlock::NativeReprobeRequired),
            ConnectionStatus::Disconnected => Some(RepoWriteBlock::Offline),
            ConnectionStatus::Connecting => Some(RepoWriteBlock::Reconnecting),
            ConnectionStatus::Connected if state.load_state != "ready" => {
                Some(RepoWriteBlock::SnapshotLoading)
            }
            ConnectionStatus::Connected
                if state.pending_branch_switch || state.pending_repo_switch =>
            {
                Some(RepoWriteBlock::ScopeSwitching)
            }
            ConnectionStatus::Connected if !state.has_repo => Some(RepoWriteBlock::NoRepo),
            ConnectionStatus::Connected => None,
        };
    }

    repo_write_block(state).filter(|block| *block != RepoWriteBlock::ReadOnly)
}
