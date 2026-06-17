//! plan_ref:
//!   - 07_network#web-ws-runtime
//!   - 09_web_thin_client_ledger#web-edit-intent
//!
use super::{
    RepoWriteBlock, RepoWriteGateState, RepoWriteSignals, repo_source_control_read_block,
    repo_source_control_read_block_tracked, repo_source_control_read_block_untracked,
    repo_write_block,
};
use crate::api::{ConnectionStatus, WsService};
use crate::hooks::use_core::PendingBranchTarget;
use deve_core::models::PeerId;
use leptos::prelude::{GetUntracked, ReadSignal, Signal, signal};

fn gate_state(
    is_read_only: bool,
    handshake_ready: bool,
    writer_ready: bool,
) -> RepoWriteGateState<'static> {
    RepoWriteGateState {
        connection_status: ConnectionStatus::Connected,
        load_state: "ready",
        is_read_only,
        node_role_probe_failed: false,
        node_role_readable: true,
        handshake_ready,
        writer_ready,
        has_repo: true,
        pending_branch_switch: false,
        pending_repo_switch: false,
    }
}

fn gate_state_with_status(connection_status: ConnectionStatus) -> RepoWriteGateState<'static> {
    RepoWriteGateState {
        connection_status,
        load_state: "ready",
        is_read_only: false,
        node_role_probe_failed: false,
        node_role_readable: true,
        handshake_ready: true,
        writer_ready: true,
        has_repo: true,
        pending_branch_switch: false,
        pending_repo_switch: false,
    }
}

fn read_signals(active_branch: ReadSignal<Option<PeerId>>, is_spectator: bool) -> RepoWriteSignals {
    let (load_state, _) = signal("ready".to_string());
    let (handshake_ready, _) = signal(false);
    let (current_repo_id, _) = signal(Some("repo-a".to_string()));
    let (current_scope_nonce, _) = signal(7u64);
    let (pending_branch_switch, _) = signal(None::<PendingBranchTarget>);
    let (pending_repo_switch, _) = signal(None::<String>);

    RepoWriteSignals {
        load_state,
        is_spectator: Signal::derive(move || is_spectator),
        handshake_ready,
        current_repo_id,
        current_scope_nonce,
        active_branch,
        pending_branch_switch,
        pending_repo_switch,
    }
}

#[test]
fn repo_write_gate_requires_writer_ready() {
    assert_eq!(
        repo_write_block(gate_state(false, true, false)),
        Some(RepoWriteBlock::HandshakingRepo)
    );
}

#[test]
fn repo_write_gate_requires_node_role_readable() {
    assert_eq!(
        repo_write_block(RepoWriteGateState {
            node_role_readable: false,
            ..gate_state(false, true, true)
        }),
        Some(RepoWriteBlock::HandshakingRepo)
    );
}

#[test]
fn repo_write_gate_reports_node_role_probe_failure() {
    assert_eq!(
        repo_write_block(RepoWriteGateState {
            node_role_probe_failed: true,
            node_role_readable: false,
            ..gate_state(false, true, true)
        }),
        Some(RepoWriteBlock::NativeReprobeRequired)
    );
}

#[test]
fn repo_write_gate_reports_node_role_probe_failure_before_snapshot_loading() {
    assert_eq!(
        repo_write_block(RepoWriteGateState {
            load_state: "loading",
            node_role_probe_failed: true,
            node_role_readable: false,
            ..gate_state(false, true, true)
        }),
        Some(RepoWriteBlock::NativeReprobeRequired)
    );
}

#[test]
fn repo_write_gate_blocks_remote_branches_as_read_only() {
    assert_eq!(
        repo_write_block(gate_state(true, true, true)),
        Some(RepoWriteBlock::ReadOnly)
    );
}

#[test]
fn repo_write_gate_allows_ready_local_repo() {
    assert_eq!(repo_write_block(gate_state(false, true, true)), None);
}

#[test]
fn repo_write_gate_blocks_native_session_pending() {
    assert_eq!(
        repo_write_block(gate_state_with_status(
            ConnectionStatus::NativeSessionPending
        )),
        Some(RepoWriteBlock::NativeSessionPending)
    );
}

#[test]
fn repo_write_gate_blocks_native_recovery_states() {
    for (status, block) in [
        (
            ConnectionStatus::NativeBootstrapInvalid,
            RepoWriteBlock::NativeBootstrapInvalid,
        ),
        (
            ConnectionStatus::NativeServiceOffline,
            RepoWriteBlock::NativeServiceOffline,
        ),
        (
            ConnectionStatus::NativeReprobeRequired,
            RepoWriteBlock::NativeReprobeRequired,
        ),
    ] {
        assert_eq!(
            repo_write_block(gate_state_with_status(status)),
            Some(block)
        );
    }
}

#[test]
fn repo_source_control_read_gate_blocks_native_recovery_states() {
    for (status, block) in [
        (
            ConnectionStatus::NativeBootstrapInvalid,
            RepoWriteBlock::NativeBootstrapInvalid,
        ),
        (
            ConnectionStatus::NativeSessionPending,
            RepoWriteBlock::NativeSessionPending,
        ),
        (
            ConnectionStatus::NativeServiceOffline,
            RepoWriteBlock::NativeServiceOffline,
        ),
        (
            ConnectionStatus::NativeReprobeRequired,
            RepoWriteBlock::NativeReprobeRequired,
        ),
    ] {
        assert_eq!(
            repo_source_control_read_block(gate_state_with_status(status)),
            Some(block)
        );
    }
}

#[test]
fn repo_source_control_read_gate_allows_remote_branch_reads() {
    assert_eq!(
        repo_source_control_read_block(gate_state(true, true, true)),
        None
    );
}

#[test]
fn repo_source_control_read_gate_allows_remote_branch_reads_without_writer_handshake() {
    assert_eq!(
        repo_source_control_read_block(gate_state(true, false, false)),
        None
    );
}

#[test]
fn repo_source_control_read_gate_allows_remote_branch_reads_without_node_role() {
    assert_eq!(
        repo_source_control_read_block(RepoWriteGateState {
            node_role_readable: false,
            ..gate_state(true, false, false)
        }),
        None
    );
}

#[test]
fn source_control_read_wrapper_treats_active_branch_as_readonly_untracked() {
    let runtime = leptos::reactive::owner::Owner::new();
    runtime.set();
    let ws = WsService::new_for_test(ConnectionStatus::Connected);
    let (active_branch, _) = signal(Some(PeerId::new("remote-peer")));

    assert_eq!(
        repo_source_control_read_block_untracked(&ws, read_signals(active_branch, false)),
        None
    );
}

#[test]
fn source_control_read_wrapper_treats_active_branch_as_readonly_tracked() {
    let runtime = leptos::reactive::owner::Owner::new();
    runtime.set();
    let ws = WsService::new_for_test(ConnectionStatus::Connected);
    let (active_branch, _) = signal(Some(PeerId::new("remote-peer")));
    let signals = read_signals(active_branch, false);
    let read_block = Signal::derive(move || repo_source_control_read_block_tracked(&ws, signals));

    assert_eq!(read_block.get_untracked(), None);
}

#[test]
fn repo_source_control_read_gate_requires_node_role_for_local_refresh() {
    assert_eq!(
        repo_source_control_read_block(RepoWriteGateState {
            node_role_readable: false,
            ..gate_state(false, true, true)
        }),
        Some(RepoWriteBlock::HandshakingRepo)
    );
}

#[test]
fn repo_source_control_read_gate_reports_node_role_probe_failure_before_read_only() {
    assert_eq!(
        repo_source_control_read_block(RepoWriteGateState {
            is_read_only: true,
            node_role_probe_failed: true,
            node_role_readable: false,
            ..gate_state(false, true, true)
        }),
        Some(RepoWriteBlock::NativeReprobeRequired)
    );
}

#[test]
fn repo_source_control_read_gate_reports_node_role_probe_failure_before_snapshot_loading() {
    assert_eq!(
        repo_source_control_read_block(RepoWriteGateState {
            load_state: "loading",
            node_role_probe_failed: true,
            node_role_readable: false,
            ..gate_state(false, true, true)
        }),
        Some(RepoWriteBlock::NativeReprobeRequired)
    );
}
