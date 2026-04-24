//! plan_ref:
//!   - 05_network#server-ws-runtime
//!   - 06_repository#repo-scope-runtime

use deve_core::models::{PeerId, RepoId};
use deve_core::protocol::{ServerError, ServerErrorCode, ServerMessage};
use tokio::sync::mpsc;

pub(super) struct KeyDenied {
    pub(super) repo_id: Option<RepoId>,
    pub(super) scope_nonce: u64,
    pub(super) branch: Option<PeerId>,
    pub(super) error: ServerError,
}

pub(super) async fn recv_protocol_error(
    rx: &mut mpsc::Receiver<ServerMessage>,
) -> (ServerErrorCode, Option<u64>) {
    match rx.recv().await {
        Some(ServerMessage::ProtocolError {
            error, scope_nonce, ..
        }) => (error.code, scope_nonce),
        other => panic!("expected ProtocolError, got {:?}", other),
    }
}

pub(super) async fn assert_key_provide(
    rx: &mut mpsc::Receiver<ServerMessage>,
    expected_repo_id: RepoId,
    expected_scope_nonce: u64,
    expected_branch: Option<PeerId>,
) {
    match rx.recv().await {
        Some(ServerMessage::KeyProvide {
            repo_id,
            scope_nonce,
            branch,
            ..
        }) => {
            assert_eq!(repo_id, expected_repo_id);
            assert_eq!(scope_nonce, expected_scope_nonce);
            assert_eq!(branch, expected_branch);
        }
        other => panic!("expected KeyProvide, got {:?}", other),
    }
}

pub(super) async fn recv_key_denied(rx: &mut mpsc::Receiver<ServerMessage>) -> KeyDenied {
    match rx.recv().await {
        Some(ServerMessage::KeyDenied {
            repo_id,
            scope_nonce,
            branch,
            error,
        }) => KeyDenied {
            repo_id,
            scope_nonce,
            branch,
            error,
        },
        other => panic!("expected KeyDenied, got {:?}", other),
    }
}
