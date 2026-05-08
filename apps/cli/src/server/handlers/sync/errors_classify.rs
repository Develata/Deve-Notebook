//! plan_ref:
//!   - 05_network#server-ws-runtime
//!
//! Sync protocol error classification.

use crate::server::error_classify::{
    is_db_locked, is_invalid_sync_payload, is_repo_context_invalid, is_repo_not_selected,
    is_repo_route_mismatch, is_stale_scope, is_storage_corruption, is_storage_not_found,
};
use deve_core::protocol::ServerErrorCode;

pub(super) fn classify_failure_code(detail: &str) -> ServerErrorCode {
    let lower = detail.to_ascii_lowercase();
    if is_repo_not_selected(&lower) {
        return ServerErrorCode::SyncRepoUnbound;
    }
    if is_repo_route_mismatch(&lower) {
        return ServerErrorCode::SyncRepoRouteMismatch;
    }
    if is_stale_scope(&lower) {
        return ServerErrorCode::ScStaleScope;
    }
    if is_invalid_sync_payload(&lower) {
        return ServerErrorCode::SyncInvalidPayload;
    }
    if lower.contains("decrypt") || lower.contains("aead") {
        return ServerErrorCode::SyncDecryptFailed;
    }
    if is_storage_not_found(&lower) {
        return ServerErrorCode::StorageNotFound;
    }
    if is_db_locked(&lower) {
        return ServerErrorCode::StorageDbLocked;
    }
    if is_storage_corruption(&lower) {
        return ServerErrorCode::StoragePersistFailed;
    }
    if is_repo_context_invalid(&lower) {
        return ServerErrorCode::ScRepoContextInvalid;
    }
    ServerErrorCode::RequestFailed
}

#[cfg(test)]
#[path = "errors_classify_test.rs"]
mod tests;
