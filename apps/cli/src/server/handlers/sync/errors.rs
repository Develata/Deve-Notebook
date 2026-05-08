//! plan_ref:
//!   - 05_network#server-ws-runtime
//!
//! Sync protocol error response helpers.

#[path = "errors_classify.rs"]
mod classify;

use crate::server::channel::DualChannel;
use deve_core::protocol::{ServerError, ServerErrorCode};
use deve_core::sync::engine::handshake::HandshakeError;

fn send(
    ch: &DualChannel,
    code: ServerErrorCode,
    detail: impl Into<String>,
    scope_nonce: Option<u64>,
) {
    ch.send_protocol_error_with_scope_nonce(ServerError::with_detail(code, detail), scope_nonce);
}

pub(super) fn request_failed(
    ch: &DualChannel,
    detail: impl Into<String>,
    scope_nonce: Option<u64>,
) {
    send(ch, ServerErrorCode::RequestFailed, detail, scope_nonce);
}

pub(super) fn classified_failure(
    ch: &DualChannel,
    detail: impl Into<String>,
    scope_nonce: Option<u64>,
) {
    let detail = detail.into();
    send(
        ch,
        classify::classify_failure_code(&detail),
        detail,
        scope_nonce,
    );
}

pub(super) fn handshake_failed(ch: &DualChannel, err: HandshakeError, scope_nonce: Option<u64>) {
    send(
        ch,
        handshake_failure_code(&err),
        format!("Handshake failed: {err}"),
        scope_nonce,
    );
}

fn handshake_failure_code(err: &HandshakeError) -> ServerErrorCode {
    if err.is_peer_auth_failure() {
        ServerErrorCode::SyncPeerUnauthenticated
    } else {
        ServerErrorCode::SyncInvalidPayload
    }
}

pub(super) fn storage_persist_failed(
    ch: &DualChannel,
    detail: impl Into<String>,
    scope_nonce: Option<u64>,
) {
    send(
        ch,
        ServerErrorCode::StoragePersistFailed,
        detail,
        scope_nonce,
    );
}

pub(super) fn sync_apply_failed(
    ch: &DualChannel,
    detail: impl Into<String>,
    scope_nonce: Option<u64>,
) {
    let detail = detail.into();
    send(
        ch,
        classify::classify_failure_code(&detail),
        detail,
        scope_nonce,
    );
}

pub(super) fn sync_peer_unauthenticated(
    ch: &DualChannel,
    detail: impl Into<String>,
    scope_nonce: Option<u64>,
) {
    send(
        ch,
        ServerErrorCode::SyncPeerUnauthenticated,
        detail,
        scope_nonce,
    );
}

#[cfg(test)]
mod tests {
    use super::handshake_failure_code;
    use deve_core::models::PeerId;
    use deve_core::protocol::ServerErrorCode;
    use deve_core::sync::engine::handshake::HandshakeError;

    #[test]
    fn maps_handshake_auth_errors_without_string_matching() {
        let peer_mismatch = HandshakeError::PeerIdMismatch {
            claimed: PeerId::new("claimed"),
            derived: "derived".to_string(),
        };

        assert_eq!(
            handshake_failure_code(&peer_mismatch),
            ServerErrorCode::SyncPeerUnauthenticated
        );
        assert_eq!(
            handshake_failure_code(&HandshakeError::InvalidSignature),
            ServerErrorCode::SyncPeerUnauthenticated
        );
    }

    #[test]
    fn maps_handshake_vector_encoding_failure_to_invalid_payload() {
        let serde_err = serde_json::from_str::<serde_json::Value>("{").expect_err("invalid json");

        assert_eq!(
            handshake_failure_code(&HandshakeError::VectorSerialization(serde_err)),
            ServerErrorCode::SyncInvalidPayload
        );
    }
}
