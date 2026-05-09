//! plan_ref:
//!   - 06_repository#repo-scope-runtime
//!
//! Docs operation protocol error response helpers.

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

pub(super) fn request_failed_scoped(
    ch: &DualChannel,
    detail: impl Into<String>,
    scope_nonce: Option<u64>,
) {
    send(ch, ServerErrorCode::RequestFailed, detail, scope_nonce);
}

pub(super) fn remote_branch_readonly_scoped(ch: &DualChannel, scope_nonce: Option<u64>) {
    ch.send_protocol_error_with_scope_nonce(
        ServerError::new(ServerErrorCode::ScRemoteBranchReadonly),
        scope_nonce,
    );
}

pub(super) fn storage_not_found_scoped(
    ch: &DualChannel,
    detail: impl Into<String>,
    scope_nonce: Option<u64>,
) {
    send(ch, ServerErrorCode::StorageNotFound, detail, scope_nonce);
}

pub(super) fn storage_conflict_scoped(
    ch: &DualChannel,
    detail: impl Into<String>,
    scope_nonce: Option<u64>,
) {
    send(ch, ServerErrorCode::StorageConflict, detail, scope_nonce);
}

pub(super) fn storage_persist_failed_scoped(
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

pub(super) fn projection_refresh_failed_scoped(
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

pub(super) fn classified_failure_scoped(
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
