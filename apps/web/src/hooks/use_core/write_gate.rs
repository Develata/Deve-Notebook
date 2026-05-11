//! plan_ref:
//!   - 05_network#web-ws-runtime
//!   - 06_repository#repo-scope-runtime
//!   - 16_web_thin_client_ledger#web-edit-intent
//!
use crate::api::WsService;
use crate::hooks::use_core::PendingBranchTarget;
use crate::hooks::use_core::types::CoreState;
use deve_core::models::PeerId;
use leptos::prelude::{Get, GetUntracked, ReadSignal, Signal};

mod logic;
#[derive(Clone, Copy)]
pub(crate) struct RepoWriteSignals {
    pub load_state: ReadSignal<String>,
    pub is_spectator: Signal<bool>,
    pub handshake_ready: ReadSignal<bool>,
    pub current_repo_id: ReadSignal<Option<String>>,
    pub current_scope_nonce: ReadSignal<u64>,
    pub active_branch: ReadSignal<Option<PeerId>>,
    pub pending_branch_switch: ReadSignal<Option<PendingBranchTarget>>,
    pub pending_repo_switch: ReadSignal<Option<String>>,
}

pub(crate) use self::logic::{
    RepoWriteBlock, RepoWriteGateState, repo_source_control_read_block, repo_write_block,
};

pub(crate) fn repo_write_block_untracked(
    ws: &WsService,
    signals: RepoWriteSignals,
) -> Option<RepoWriteBlock> {
    let repo_id = signals.current_repo_id.get_untracked();
    let load_state = signals.load_state.get_untracked();
    let scope_nonce = signals.current_scope_nonce.get_untracked();
    let readiness = ws.native_runtime_readiness_for_untracked(
        repo_id.as_deref(),
        Some(scope_nonce),
        signals.handshake_ready.get_untracked(),
    );
    repo_write_block(RepoWriteGateState {
        connection_status: ws.status.get_untracked(),
        load_state: &load_state,
        is_read_only: signals.is_spectator.get_untracked()
            || signals.active_branch.get_untracked().is_some(),
        node_role_probe_failed: ws.node_role_probe_failed.get_untracked(),
        node_role_readable: readiness.node_role_readable,
        handshake_ready: readiness.repo_handshake_complete,
        writer_ready: readiness.writer_ready,
        has_repo: repo_id.is_some(),
        pending_branch_switch: signals.pending_branch_switch.get_untracked().is_some(),
        pending_repo_switch: signals.pending_repo_switch.get_untracked().is_some(),
    })
}

pub(crate) fn repo_write_block_tracked(
    ws: &WsService,
    signals: RepoWriteSignals,
) -> Option<RepoWriteBlock> {
    let repo_id = signals.current_repo_id.get();
    let load_state = signals.load_state.get();
    let scope_nonce = signals.current_scope_nonce.get();
    let readiness = ws.native_runtime_readiness_for(
        repo_id.as_deref(),
        Some(scope_nonce),
        signals.handshake_ready.get(),
    );
    repo_write_block(RepoWriteGateState {
        connection_status: ws.status.get(),
        load_state: &load_state,
        is_read_only: signals.is_spectator.get() || signals.active_branch.get().is_some(),
        node_role_probe_failed: ws.node_role_probe_failed.get(),
        node_role_readable: readiness.node_role_readable,
        handshake_ready: readiness.repo_handshake_complete,
        writer_ready: readiness.writer_ready,
        has_repo: repo_id.is_some(),
        pending_branch_switch: signals.pending_branch_switch.get().is_some(),
        pending_repo_switch: signals.pending_repo_switch.get().is_some(),
    })
}

pub(crate) fn repo_source_control_read_block_untracked(
    ws: &WsService,
    signals: RepoWriteSignals,
) -> Option<RepoWriteBlock> {
    let repo_id = signals.current_repo_id.get_untracked();
    let load_state = signals.load_state.get_untracked();
    let scope_nonce = signals.current_scope_nonce.get_untracked();
    let readiness = ws.native_runtime_readiness_for_untracked(
        repo_id.as_deref(),
        Some(scope_nonce),
        signals.handshake_ready.get_untracked(),
    );
    repo_source_control_read_block(RepoWriteGateState {
        connection_status: ws.status.get_untracked(),
        load_state: &load_state,
        is_read_only: signals.is_spectator.get_untracked(),
        node_role_probe_failed: ws.node_role_probe_failed.get_untracked(),
        node_role_readable: readiness.node_role_readable,
        handshake_ready: readiness.repo_handshake_complete,
        writer_ready: readiness.writer_ready,
        has_repo: repo_id.is_some(),
        pending_branch_switch: signals.pending_branch_switch.get_untracked().is_some(),
        pending_repo_switch: signals.pending_repo_switch.get_untracked().is_some(),
    })
}

pub(crate) fn repo_source_control_read_block_tracked(
    ws: &WsService,
    signals: RepoWriteSignals,
) -> Option<RepoWriteBlock> {
    let repo_id = signals.current_repo_id.get();
    let load_state = signals.load_state.get();
    let scope_nonce = signals.current_scope_nonce.get();
    let readiness = ws.native_runtime_readiness_for(
        repo_id.as_deref(),
        Some(scope_nonce),
        signals.handshake_ready.get(),
    );
    repo_source_control_read_block(RepoWriteGateState {
        connection_status: ws.status.get(),
        load_state: &load_state,
        is_read_only: signals.is_spectator.get(),
        node_role_probe_failed: ws.node_role_probe_failed.get(),
        node_role_readable: readiness.node_role_readable,
        handshake_ready: readiness.repo_handshake_complete,
        writer_ready: readiness.writer_ready,
        has_repo: repo_id.is_some(),
        pending_branch_switch: signals.pending_branch_switch.get().is_some(),
        pending_repo_switch: signals.pending_repo_switch.get().is_some(),
    })
}

pub(crate) fn repo_write_allowed_for_core_tracked(core: &CoreState) -> bool {
    repo_write_block_tracked(
        &core.ws,
        RepoWriteSignals {
            load_state: core.load_state,
            is_spectator: core.is_spectator,
            handshake_ready: core.handshake_ready,
            current_repo_id: core.current_repo_id,
            current_scope_nonce: core.current_scope_nonce,
            active_branch: core.active_branch,
            pending_branch_switch: core.pending_branch_switch,
            pending_repo_switch: core.pending_repo_switch,
        },
    )
    .is_none()
}

#[cfg(test)]
mod tests;
