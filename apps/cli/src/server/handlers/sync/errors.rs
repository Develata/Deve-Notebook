use crate::server::channel::DualChannel;
use deve_core::protocol::{ServerError, ServerErrorCode};

fn send(ch: &DualChannel, code: ServerErrorCode, detail: impl Into<String>) {
    ch.send_protocol_error(ServerError::with_detail(code, detail));
}

pub(super) fn engine_unavailable(ch: &DualChannel) {
    send(
        ch,
        ServerErrorCode::RequestFailed,
        "Failed to get or create sync engine",
    );
}

pub(super) fn request_failed(ch: &DualChannel, detail: impl Into<String>) {
    send(ch, ServerErrorCode::RequestFailed, detail);
}

pub(super) fn storage_persist_failed(ch: &DualChannel, detail: impl Into<String>) {
    send(ch, ServerErrorCode::StoragePersistFailed, detail);
}

pub(super) fn sync_apply_failed(ch: &DualChannel, detail: impl Into<String>) {
    let detail = detail.into();
    let lower = detail.to_ascii_lowercase();
    let code = if lower.contains("decrypt") || lower.contains("aead") {
        ServerErrorCode::SyncDecryptFailed
    } else {
        ServerErrorCode::RequestFailed
    };
    send(ch, code, detail);
}
