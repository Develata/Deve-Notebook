//! plan_ref:
//!   - 06_repository#repo-scope-runtime
//!
//! Common repository-scope error classification.

use crate::server::error_classify::{
    is_db_locked, is_remote_exact_selector_mismatch, is_repo_context_invalid, is_repo_not_selected,
    is_stale_scope, is_storage_corruption, is_storage_not_found,
};
use deve_core::protocol::ServerErrorCode;

pub(super) fn classify_common_scope_code(detail: &str) -> Option<ServerErrorCode> {
    let lower = detail.to_ascii_lowercase();
    if is_repo_not_selected(&lower) {
        return Some(ServerErrorCode::ScRepoNotSelected);
    }
    if is_storage_not_found(&lower) {
        return Some(ServerErrorCode::StorageNotFound);
    }
    if is_db_locked(&lower) {
        return Some(ServerErrorCode::StorageDbLocked);
    }
    if is_storage_corruption(&lower) {
        return Some(ServerErrorCode::StoragePersistFailed);
    }
    if lower.contains("tracked document projection missing") {
        return Some(ServerErrorCode::StoragePersistFailed);
    }
    if is_remote_exact_selector_mismatch(&lower) {
        return Some(ServerErrorCode::ScRepoContextInvalid);
    }
    if is_stale_scope(&lower) {
        return Some(ServerErrorCode::ScStaleScope);
    }
    if is_repo_context_invalid(&lower) {
        return Some(ServerErrorCode::ScRepoContextInvalid);
    }
    None
}
