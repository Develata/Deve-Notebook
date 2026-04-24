//! plan_ref:
//!   - 06_repository#repo-scope-runtime
//!
//! Docs operation protocol error classification.

use crate::server::repo_scope::map_repo_scope_error;
use anyhow::anyhow;
use deve_core::protocol::ServerErrorCode;

pub(super) fn classify_failure_code(detail: &str) -> ServerErrorCode {
    let repo_scope_code = map_repo_scope_error(anyhow!(detail.to_string())).code;
    if repo_scope_code != ServerErrorCode::RequestFailed {
        return repo_scope_code;
    }
    let lower = detail.to_ascii_lowercase();
    if lower.contains("source not found:") {
        return ServerErrorCode::StorageNotFound;
    }
    if is_projection_breakage(&lower) {
        return ServerErrorCode::StoragePersistFailed;
    }
    if is_conflict(&lower) {
        return ServerErrorCode::StorageConflict;
    }
    ServerErrorCode::RequestFailed
}

fn is_projection_breakage(lower: &str) -> bool {
    lower.contains("node meta missing")
        || lower.contains("tracked document projection missing")
        || lower.contains("canonical node path resolution failed")
}

fn is_conflict(lower: &str) -> bool {
    lower.contains("already exists")
        || lower.contains("destination exists")
        || lower.contains("target file already exists on disk")
}

#[cfg(test)]
#[path = "errors_classify_test.rs"]
mod tests;
