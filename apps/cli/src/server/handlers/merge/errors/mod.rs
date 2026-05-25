//! plan_ref:
//!   - 04_repository#repo-scope-runtime
//!
//! Merge handler protocol error helpers.

use crate::server::channel::DualChannel;
use crate::server::error_classify::{
    is_db_locked, is_repo_context_invalid, is_stale_scope, is_storage_corruption,
    is_storage_not_found,
};
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

pub(super) fn storage_conflict(
    ch: &DualChannel,
    detail: impl Into<String>,
    scope_nonce: Option<u64>,
) {
    send(ch, ServerErrorCode::StorageConflict, detail, scope_nonce);
}

pub(super) fn storage_not_found(
    ch: &DualChannel,
    detail: impl Into<String>,
    scope_nonce: Option<u64>,
) {
    send(ch, ServerErrorCode::StorageNotFound, detail, scope_nonce);
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

pub(super) fn classified_failure(
    ch: &DualChannel,
    detail: impl Into<String>,
    scope_nonce: Option<u64>,
) {
    let detail = detail.into();
    send(ch, classify_failure_code(&detail), detail, scope_nonce);
}

fn classify_failure_code(detail: &str) -> ServerErrorCode {
    let lower = detail.to_ascii_lowercase();
    if is_storage_not_found(&lower)
        || lower.contains("no local repository matched the active remote repository")
    {
        return ServerErrorCode::StorageNotFound;
    }
    if is_db_locked(&lower) {
        return ServerErrorCode::StorageDbLocked;
    }
    if is_storage_corruption(&lower) {
        return ServerErrorCode::StoragePersistFailed;
    }
    if is_stale_scope(&lower) {
        return ServerErrorCode::ScStaleScope;
    }
    if is_repo_context_invalid(&lower) {
        return ServerErrorCode::ScRepoContextInvalid;
    }
    ServerErrorCode::RequestFailed
}

#[cfg(test)]
mod classify_tests;
