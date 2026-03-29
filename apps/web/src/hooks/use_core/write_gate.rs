use crate::api::WsService;
use crate::hooks::use_core::PendingBranchTarget;
use crate::hooks::use_core::types::CoreState;
use leptos::prelude::{Get, GetUntracked, ReadSignal, Signal};

#[path = "write_gate_logic.rs"]
mod logic;
#[derive(Clone, Copy)]
pub(crate) struct RepoWriteSignals {
    pub load_state: ReadSignal<String>,
    pub is_spectator: Signal<bool>,
    pub handshake_ready: ReadSignal<bool>,
    pub current_repo_id: ReadSignal<Option<String>>,
    pub pending_branch_switch: ReadSignal<Option<PendingBranchTarget>>,
    pub pending_repo_switch: ReadSignal<Option<String>>,
}

pub(crate) use self::logic::{RepoWriteBlock, repo_write_block};

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

pub(crate) fn repo_write_block_tracked(
    ws: &WsService,
    signals: RepoWriteSignals,
) -> Option<RepoWriteBlock> {
    let repo_id = signals.current_repo_id.get();
    repo_write_block(
        ws.status.get(),
        &signals.load_state.get(),
        signals.is_spectator.get(),
        signals.handshake_ready.get(),
        ws.writer_ready_for(repo_id.as_deref()),
        repo_id.is_some(),
        signals.pending_branch_switch.get().is_some(),
        signals.pending_repo_switch.get().is_some(),
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

pub(crate) fn repo_write_allowed_for_core_tracked(core: &CoreState) -> bool {
    repo_write_block_tracked(
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
    .is_none()
}

#[cfg(test)]
#[path = "write_gate_tests.rs"]
mod tests;
