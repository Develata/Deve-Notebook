use super::{RepoWriteBlock, repo_write_block};
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
fn repo_write_gate_reports_read_only_before_handshake() {
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
