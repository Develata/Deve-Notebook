use crate::api::ConnectionStatus;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum RepoWriteBlock {
    SessionExpired,
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

pub(crate) fn repo_write_block(
    connection_status: ConnectionStatus,
    load_state: &str,
    is_read_only: bool,
    handshake_ready: bool,
    writer_ready: bool,
    has_repo: bool,
    pending_branch_switch: bool,
    pending_repo_switch: bool,
) -> Option<RepoWriteBlock> {
    match connection_status {
        ConnectionStatus::Unauthorized => Some(RepoWriteBlock::SessionExpired),
        ConnectionStatus::Disconnected => Some(RepoWriteBlock::Offline),
        ConnectionStatus::Connecting => Some(RepoWriteBlock::Reconnecting),
        ConnectionStatus::Connected if load_state != "ready" => {
            Some(RepoWriteBlock::SnapshotLoading)
        }
        ConnectionStatus::Connected if is_read_only => Some(RepoWriteBlock::ReadOnly),
        ConnectionStatus::Connected if pending_branch_switch || pending_repo_switch => {
            Some(RepoWriteBlock::ScopeSwitching)
        }
        ConnectionStatus::Connected if !has_repo => Some(RepoWriteBlock::NoRepo),
        ConnectionStatus::Connected if !handshake_ready || !writer_ready => {
            Some(RepoWriteBlock::HandshakingRepo)
        }
        ConnectionStatus::Connected => None,
    }
}

pub(crate) fn repo_source_control_read_block(
    connection_status: ConnectionStatus,
    load_state: &str,
    is_read_only: bool,
    handshake_ready: bool,
    writer_ready: bool,
    has_repo: bool,
    pending_branch_switch: bool,
    pending_repo_switch: bool,
) -> Option<RepoWriteBlock> {
    if is_read_only {
        return match connection_status {
            ConnectionStatus::Unauthorized => Some(RepoWriteBlock::SessionExpired),
            ConnectionStatus::Disconnected => Some(RepoWriteBlock::Offline),
            ConnectionStatus::Connecting => Some(RepoWriteBlock::Reconnecting),
            ConnectionStatus::Connected if load_state != "ready" => {
                Some(RepoWriteBlock::SnapshotLoading)
            }
            ConnectionStatus::Connected if pending_branch_switch || pending_repo_switch => {
                Some(RepoWriteBlock::ScopeSwitching)
            }
            ConnectionStatus::Connected if !has_repo => Some(RepoWriteBlock::NoRepo),
            ConnectionStatus::Connected => None,
        };
    }

    repo_write_block(
        connection_status,
        load_state,
        is_read_only,
        handshake_ready,
        writer_ready,
        has_repo,
        pending_branch_switch,
        pending_repo_switch,
    )
    .filter(|block| *block != RepoWriteBlock::ReadOnly)
}
