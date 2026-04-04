use super::{RepoWriteBlock, repo_source_control_read_block, repo_write_block};
use crate::api::ConnectionStatus;

#[test]
fn repo_write_gate_requires_writer_ready() {
    assert_eq!(
        repo_write_block(
            ConnectionStatus::Connected,
            "ready",
            false,
            true,
            false,
            true,
            false,
            false
        ),
        Some(RepoWriteBlock::HandshakingRepo)
    );
}

#[test]
fn repo_write_gate_blocks_remote_branches_as_read_only() {
    assert_eq!(
        repo_write_block(
            ConnectionStatus::Connected,
            "ready",
            true,
            true,
            true,
            true,
            false,
            false
        ),
        Some(RepoWriteBlock::ReadOnly)
    );
}

#[test]
fn repo_write_gate_allows_ready_local_repo() {
    assert_eq!(
        repo_write_block(
            ConnectionStatus::Connected,
            "ready",
            false,
            true,
            true,
            true,
            false,
            false
        ),
        None
    );
}

#[test]
fn repo_source_control_read_gate_allows_remote_branch_reads() {
    assert_eq!(
        repo_source_control_read_block(
            ConnectionStatus::Connected,
            "ready",
            true,
            true,
            true,
            true,
            false,
            false
        ),
        None
    );
}

#[test]
fn repo_source_control_read_gate_allows_remote_branch_reads_without_writer_handshake() {
    assert_eq!(
        repo_source_control_read_block(
            ConnectionStatus::Connected,
            "ready",
            true,
            false,
            false,
            true,
            false,
            false
        ),
        None
    );
}
