//! plan_ref:
//!   - 06_repository#repo-scope-runtime
//!
//! Repo scope error taxonomy and protocol mapping.

use crate::server::error_classify::{
    is_db_locked, is_remote_exact_selector_mismatch, is_repo_context_invalid, is_repo_not_selected,
    is_stale_scope, is_storage_corruption, is_storage_not_found,
};
use anyhow::Error;
use deve_core::protocol::{ServerError, ServerErrorCode};

pub const STALE_REMOTE_SCOPE_PREFIX: &str = "stale remote scope:";

pub fn stale_remote_scope_detail(detail: impl AsRef<str>) -> String {
    format!("{STALE_REMOTE_SCOPE_PREFIX} {}", detail.as_ref())
}

pub fn map_repo_scope_error(error: Error) -> ServerError {
    let detail = error.to_string();
    let lower = detail.to_ascii_lowercase();
    if is_repo_not_selected(&lower) {
        return ServerError::with_detail(ServerErrorCode::SyncRepoUnbound, detail);
    }
    if is_storage_not_found(&lower) {
        return ServerError::with_detail(ServerErrorCode::StorageNotFound, detail);
    }
    if is_db_locked(&lower) {
        return ServerError::with_detail(ServerErrorCode::StorageDbLocked, detail);
    }
    if is_storage_corruption(&lower) {
        return ServerError::with_detail(ServerErrorCode::StoragePersistFailed, detail);
    }
    if is_remote_exact_selector_mismatch(&lower) {
        return ServerError::with_detail(ServerErrorCode::ScRepoContextInvalid, detail);
    }
    if is_stale_scope(&lower) {
        return ServerError::with_detail(ServerErrorCode::ScStaleScope, detail);
    }
    if is_repo_context_invalid(&lower) {
        return ServerError::with_detail(ServerErrorCode::ScRepoContextInvalid, detail);
    }
    ServerError::with_detail(ServerErrorCode::RequestFailed, detail)
}

#[cfg(test)]
#[path = "repo_scope_error_test.rs"]
mod tests;
