//! plan_ref:
//!   - 05_network#web-ws-runtime
//!   - 16_web_thin_client_ledger#web-edit-intent
//!
use super::{RepoWriteBlock, RepoWriteGateState, repo_source_control_read_block, repo_write_block};
use crate::api::ConnectionStatus;

fn gate_state(
    is_read_only: bool,
    handshake_ready: bool,
    writer_ready: bool,
) -> RepoWriteGateState<'static> {
    RepoWriteGateState {
        connection_status: ConnectionStatus::Connected,
        load_state: "ready",
        is_read_only,
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
        handshake_ready: true,
        writer_ready: true,
        has_repo: true,
        pending_branch_switch: false,
        pending_repo_switch: false,
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
fn repo_write_gate_blocks_native_reprobe_required() {
    assert_eq!(
        repo_write_block(gate_state_with_status(
            ConnectionStatus::NativeReprobeRequired
        )),
        Some(RepoWriteBlock::NativeReprobeRequired)
    );
}

#[test]
fn repo_source_control_read_gate_blocks_native_recovery_states() {
    assert_eq!(
        repo_source_control_read_block(gate_state_with_status(
            ConnectionStatus::NativeServiceOffline
        )),
        Some(RepoWriteBlock::NativeServiceOffline)
    );
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
