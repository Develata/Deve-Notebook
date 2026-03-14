use crate::server::channel::DualChannel;
use deve_core::protocol::{ServerError, ServerErrorCode};

fn send(ch: &DualChannel, code: ServerErrorCode, detail: impl Into<String>) {
    ch.send_protocol_error(ServerError::with_detail(code, detail));
}

pub(super) fn engine_unavailable(ch: &DualChannel) {
    send(
        ch,
        ServerErrorCode::StoragePersistFailed,
        "Failed to get or create sync engine",
    );
}

pub(super) fn request_failed(ch: &DualChannel, detail: impl Into<String>) {
    send(ch, ServerErrorCode::RequestFailed, detail);
}

pub(super) fn classified_failure(ch: &DualChannel, detail: impl Into<String>) {
    let detail = detail.into();
    send(ch, classify_failure_code(&detail), detail);
}

pub(super) fn storage_persist_failed(ch: &DualChannel, detail: impl Into<String>) {
    send(ch, ServerErrorCode::StoragePersistFailed, detail);
}

pub(super) fn sync_apply_failed(ch: &DualChannel, detail: impl Into<String>) {
    let detail = detail.into();
    send(ch, classify_failure_code(&detail), detail);
}

fn classify_failure_code(detail: &str) -> ServerErrorCode {
    let lower = detail.to_ascii_lowercase();
    if lower.contains("decrypt") || lower.contains("aead") {
        return ServerErrorCode::SyncDecryptFailed;
    }
    if lower.contains("database already open")
        || lower.contains("cannot acquire lock")
        || lower.contains("db locked")
        || lower.contains("database is locked")
    {
        return ServerErrorCode::StorageDbLocked;
    }
    if lower.contains("signature")
        || lower.contains("verify")
        || lower.contains("public key")
        || lower.contains("unauthenticated")
    {
        return ServerErrorCode::SyncPeerUnauthenticated;
    }
    if lower.contains("snapshot")
        || lower.contains("sync payload")
        || lower.contains("sync response")
        || lower.contains("sync engine")
    {
        return ServerErrorCode::StoragePersistFailed;
    }
    ServerErrorCode::RequestFailed
}

#[cfg(test)]
mod tests {
    use super::classify_failure_code;
    use deve_core::protocol::ServerErrorCode;

    #[test]
    fn classifies_signature_failures_as_peer_unauthenticated() {
        assert_eq!(
            classify_failure_code("Handshake failed: signature verification failed"),
            ServerErrorCode::SyncPeerUnauthenticated
        );
    }

    #[test]
    fn classifies_snapshot_generation_as_storage_persist_failed() {
        assert_eq!(
            classify_failure_code("Failed to generate snapshot for repo x"),
            ServerErrorCode::StoragePersistFailed
        );
    }
}
