//! plan_ref:
//!   - 04_repository#repo-scope-runtime
//!
//! Repository-scope source-control error mapping tests.

use super::*;
use crate::server::repo_scope::RepoScopeFailure;

#[test]
fn maps_repo_scope_miss_to_repo_not_selected() {
    let err = map_repo_scope_error(anyhow::anyhow!(
        "Active repository not selected for current session"
    ));
    assert_eq!(err.code, ServerErrorCode::ScRepoNotSelected);
}

#[test]
fn maps_repo_selector_mismatch_to_repo_context_invalid() {
    let err = map_repo_scope_error(anyhow::anyhow!(
        "Repo selector mismatch: repo_id resolved to default, repo_name resolved to test"
    ));
    assert_eq!(err.code, ServerErrorCode::ScRepoContextInvalid);
}

#[test]
fn maps_local_selector_miss_to_repo_context_invalid() {
    let err = map_repo_scope_error(anyhow::anyhow!(
        "Local repository selector not resolved for test"
    ));
    assert_eq!(err.code, ServerErrorCode::ScRepoContextInvalid);
}

#[test]
fn maps_scope_missing_repo_to_storage_not_found() {
    let err = map_repo_scope_error(anyhow::anyhow!("Repository not found: default"));
    assert_eq!(err.code, ServerErrorCode::StorageNotFound);
}

#[test]
fn maps_scope_locked_db_to_storage_db_locked() {
    let err = map_repo_scope_error(anyhow::anyhow!(
        "Database already open. Cannot acquire lock."
    ));
    assert_eq!(err.code, ServerErrorCode::StorageDbLocked);
}

#[test]
fn maps_scope_ambiguous_local_selector_to_repo_context_invalid() {
    let err = map_repo_scope_error(anyhow::anyhow!("Ambiguous local repository selector: wiki"));
    assert_eq!(err.code, ServerErrorCode::ScRepoContextInvalid);
}

#[test]
fn maps_scope_remote_bootstrap_drift_to_repo_context_invalid() {
    let err = map_repo_scope_error(anyhow::anyhow!(
        "Cannot bootstrap local repo while on remote branch"
    ));
    assert_eq!(err.code, ServerErrorCode::ScRepoContextInvalid);
}

#[test]
fn maps_scope_stale_nonce_to_stale_scope() {
    let err = map_repo_scope_error(anyhow::anyhow!(
        "Browser SyncHello stale scope nonce: current_scope_nonce=9, requested_scope_nonce=7"
    ));
    assert_eq!(err.code, ServerErrorCode::ScStaleScope);
}

#[test]
fn maps_stale_remote_exact_selector_mismatch_to_repo_context_invalid() {
    let err = map_repo_scope_error(anyhow::anyhow!(
        "stale remote scope: Session repo mismatch: expected a, resolved b for exact repository selector wiki-2"
    ));
    assert_eq!(err.code, ServerErrorCode::ScRepoContextInvalid);
}

#[test]
fn maps_typed_repo_unbound_to_source_control_not_selected() {
    let err = map_repo_scope_error(RepoScopeFailure::repo_unbound("typed repo unbound").into());
    assert_eq!(err.code, ServerErrorCode::ScRepoNotSelected);
}

#[test]
fn maps_typed_exact_selector_mismatch_to_repo_context_invalid() {
    let err = map_repo_scope_error(
        RepoScopeFailure::exact_selector_mismatch("typed exact selector mismatch").into(),
    );
    assert_eq!(err.code, ServerErrorCode::ScRepoContextInvalid);
}

#[test]
fn maps_broken_shadow_listing_to_storage_persist_failed() {
    let err = map_repo_scope_error(anyhow::anyhow!(
        "Broken shadow repo notes for peer peer-a while listing repos"
    ));
    assert_eq!(err.code, ServerErrorCode::StoragePersistFailed);
}

#[test]
fn maps_repo_decode_failures_to_storage_persist_failed() {
    let err = map_repo_scope_error(anyhow::anyhow!("failed to deserialize repo metadata"));
    assert_eq!(err.code, ServerErrorCode::StoragePersistFailed);
}

#[test]
fn maps_broken_repo_entry_to_storage_persist_failed() {
    let err = map_repo_scope_error(anyhow::anyhow!(
        "Broken repo entry \"/tmp/local/.redb\" while listing repos: invalid file stem"
    ));
    assert_eq!(err.code, ServerErrorCode::StoragePersistFailed);
}

#[test]
fn maps_missing_remote_catalog_to_storage_persist_failed() {
    let err = map_repo_scope_error(anyhow::anyhow!(
        "Broken remote repo catalog: remote repo directory missing at /tmp/ledger/remotes"
    ));
    assert_eq!(err.code, ServerErrorCode::StoragePersistFailed);
}

#[test]
fn maps_missing_selector_with_multiple_local_repos() {
    let err = map_repo_error(
        ScOp::ListChanges,
        anyhow::anyhow!("Active repository not selected: multiple local repos exist"),
    );
    assert_eq!(err.code, ServerErrorCode::ScRepoNotSelected);
}

#[test]
fn maps_repo_error_selector_mismatch_to_repo_context_invalid() {
    let err = map_repo_error(
        ScOp::ListChanges,
        anyhow::anyhow!(
            "Repo selector mismatch: repo_id resolved to default, repo_name resolved to test"
        ),
    );
    assert_eq!(err.code, ServerErrorCode::ScRepoContextInvalid);
}

#[test]
fn maps_repo_error_missing_repo_to_storage_not_found() {
    let err = map_repo_error(
        ScOp::ListChanges,
        anyhow::anyhow!("Repository not found: default"),
    );
    assert_eq!(err.code, ServerErrorCode::StorageNotFound);
}
