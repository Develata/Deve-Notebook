//! plan_ref:
//!   - 06_repository#repo-scope-runtime

use crate::server::repo_scope::map_repo_scope_error;
use deve_core::protocol::ServerErrorCode;

#[test]
fn map_repo_scope_error_marks_selector_mismatch_as_context_invalid() {
    let err = map_repo_scope_error(anyhow::anyhow!(
        "Repo selector mismatch: repo_id resolved to default, repo_name resolved to test"
    ));
    assert_eq!(err.code, ServerErrorCode::ScRepoContextInvalid);
}

#[test]
fn map_repo_scope_error_marks_missing_repo_as_not_found() {
    let err = map_repo_scope_error(anyhow::anyhow!("Repository not found: default"));
    assert_eq!(err.code, ServerErrorCode::StorageNotFound);
    let local_name_err =
        map_repo_scope_error(anyhow::anyhow!("Local repo not found for name test"));
    assert_eq!(local_name_err.code, ServerErrorCode::StorageNotFound);
}

#[test]
fn map_repo_scope_error_marks_locked_db_as_storage_db_locked() {
    let err = map_repo_scope_error(anyhow::anyhow!(
        "Database already open. Cannot acquire lock."
    ));
    assert_eq!(err.code, ServerErrorCode::StorageDbLocked);
}

#[test]
fn map_repo_scope_error_marks_stale_nonce_as_context_invalid() {
    let err = map_repo_scope_error(anyhow::anyhow!(
        "Browser SyncHello stale scope nonce: current_scope_nonce=9, requested_scope_nonce=7"
    ));
    assert_eq!(err.code, ServerErrorCode::ScRepoContextInvalid);
}

#[test]
fn map_repo_scope_error_marks_remote_bootstrap_as_context_invalid() {
    let err = map_repo_scope_error(anyhow::anyhow!(
        "Cannot bootstrap local repo while on remote branch"
    ));
    assert_eq!(err.code, ServerErrorCode::ScRepoContextInvalid);
}
