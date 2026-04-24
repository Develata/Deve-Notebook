//! plan_ref:
//!   - 05_network#server-ws-runtime
//!
//! Sync protocol error response helpers.

#[path = "errors_classify.rs"]
mod classify;

use crate::server::channel::DualChannel;
use deve_core::protocol::{ServerError, ServerErrorCode};

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
