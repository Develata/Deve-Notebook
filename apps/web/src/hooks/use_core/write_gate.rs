use crate::api::{ConnectionStatus, WsService};
use crate::hooks::use_core::PendingBranchTarget;
use crate::hooks::use_core::types::CoreState;
use leptos::prelude::{GetUntracked, ReadSignal, Signal};

#[derive(Clone, Copy)]
pub(crate) struct RepoWriteSignals {
    pub load_state: ReadSignal<String>,
    pub is_spectator: Signal<bool>,
    pub handshake_ready: ReadSignal<bool>,
    pub current_repo_id: ReadSignal<Option<String>>,
    pub pending_branch_switch: ReadSignal<Option<PendingBranchTarget>>,
    pub pending_repo_switch: ReadSignal<Option<String>>,
}

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

pub(crate) fn repo_write_block_untracked(
    ws: &WsService,
    signals: RepoWriteSignals,
) -> Option<RepoWriteBlock> {
    let repo_id = signals.current_repo_id.get_untracked();
    repo_write_block(
        ws.status.get_untracked(),
        &signals.load_state.get_untracked(),
        signals.is_spectator.get_untracked(),
        signals.handshake_ready.get_untracked(),
        ws.writer_ready_for(repo_id.as_deref()),
        repo_id.is_some(),
        signals.pending_branch_switch.get_untracked().is_some(),
        signals.pending_repo_switch.get_untracked().is_some(),
    )
}

pub(crate) fn repo_write_allowed_untracked(ws: &WsService, signals: RepoWriteSignals) -> bool {
    repo_write_block_untracked(ws, signals).is_none()
}

pub(crate) fn repo_write_allowed_for_core(core: &CoreState) -> bool {
    repo_write_allowed_untracked(
        &core.ws,
        RepoWriteSignals {
            load_state: core.load_state,
            is_spectator: core.is_spectator,
            handshake_ready: core.handshake_ready,
            current_repo_id: core.current_repo_id,
            pending_branch_switch: core.pending_branch_switch,
            pending_repo_switch: core.pending_repo_switch,
        },
    )
}

fn repo_write_block(
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

#[cfg(test)]
#[path = "write_gate_tests.rs"]
mod tests;
