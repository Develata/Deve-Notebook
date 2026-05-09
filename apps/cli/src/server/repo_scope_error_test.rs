//! plan_ref:
//!   - 06_repository#repo-scope-runtime
//!
//! Repo scope error classification regression tests.

use super::{RepoScopeFailure, map_repo_scope_error};
use deve_core::protocol::ServerErrorCode;

#[test]
fn classifies_ambiguous_remote_selector_as_repo_context_invalid() {
    let err = map_repo_scope_error(anyhow::anyhow!(
        "Ambiguous remote repository selector: shadow-wiki"
    ));
    assert_eq!(err.code, ServerErrorCode::ScRepoContextInvalid);
}

#[test]
fn classifies_remote_workspace_access_as_repo_context_invalid() {
    let err = map_repo_scope_error(anyhow::anyhow!(
        "Local workspace path requested on remote branch: wiki"
    ));
    assert_eq!(err.code, ServerErrorCode::ScRepoContextInvalid);
}

#[test]
fn classifies_stale_remote_exact_selector_mismatch_as_context_invalid() {
    let err = map_repo_scope_error(anyhow::anyhow!(
        "stale remote scope: Session repo mismatch: expected a, resolved b for exact repository selector wiki-2"
    ));
    assert_eq!(err.code, ServerErrorCode::ScRepoContextInvalid);
}

#[test]
fn maps_typed_remote_branch_unavailable_without_string_classification() {
    let err = map_repo_scope_error(RepoScopeFailure::remote_branch_unavailable("peer-a").into());
    assert_eq!(err.code, ServerErrorCode::ScStaleScope);
    assert_eq!(
        err.detail.as_deref(),
        Some("stale remote scope: Remote branch not available: peer-a")
    );
}

#[test]
fn maps_typed_exact_selector_mismatch_without_string_classification() {
    let err = map_repo_scope_error(
        RepoScopeFailure::exact_selector_mismatch("typed exact selector mismatch").into(),
    );
    assert_eq!(err.code, ServerErrorCode::ScRepoContextInvalid);
    assert_eq!(err.detail.as_deref(), Some("typed exact selector mismatch"));
}

#[test]
fn maps_typed_repo_unbound_without_string_classification() {
    let err = map_repo_scope_error(RepoScopeFailure::repo_unbound("typed repo unbound").into());
    assert_eq!(err.code, ServerErrorCode::SyncRepoUnbound);
}

#[test]
fn classifies_missing_local_repo_name_as_storage_not_found() {
    let err = map_repo_scope_error(anyhow::anyhow!("Local repo not found for name wiki"));
    assert_eq!(err.code, ServerErrorCode::StorageNotFound);
}

#[test]
fn classifies_broken_shadow_listing_as_storage_persist_failed() {
    let err = map_repo_scope_error(anyhow::anyhow!(
        "Broken shadow repo notes for peer peer-a while listing repos"
    ));
    assert_eq!(err.code, ServerErrorCode::StoragePersistFailed);
}

#[test]
fn classifies_broken_repo_entry_as_storage_persist_failed() {
    let err = map_repo_scope_error(anyhow::anyhow!(
        "Broken repo entry \"/tmp/local/.redb\" while listing repos: invalid file stem"
    ));
    assert_eq!(err.code, ServerErrorCode::StoragePersistFailed);
}

#[test]
fn classifies_missing_remote_catalog_as_storage_persist_failed() {
    let err = map_repo_scope_error(anyhow::anyhow!(
        "Broken remote repo catalog: remote repo directory missing at /tmp/ledger/remotes"
    ));
    assert_eq!(err.code, ServerErrorCode::StoragePersistFailed);
}

#[test]
fn classifies_broken_remote_repo_as_storage_persist_failed() {
    let err = map_repo_scope_error(anyhow::anyhow!(
        "Broken remote repo wiki while resolving current repo URL before branch switch: repository URL missing"
    ));
    assert_eq!(err.code, ServerErrorCode::StoragePersistFailed);
}

#[test]
fn classifies_missing_local_selector_as_repo_context_invalid() {
    let err = map_repo_scope_error(anyhow::anyhow!(
        "Local repository selector not resolved for stale-name"
    ));
    assert_eq!(err.code, ServerErrorCode::ScRepoContextInvalid);
}

#[test]
fn classifies_repo_metadata_decode_failure_as_storage_persist_failed() {
    let err = map_repo_scope_error(anyhow::anyhow!("failed to deserialize repo metadata"));
    assert_eq!(err.code, ServerErrorCode::StoragePersistFailed);
}

#[test]
fn classifies_local_catalog_stat_failure_as_storage_persist_failed() {
    let err = map_repo_scope_error(anyhow::anyhow!(
        "Failed to stat local repo directory while resolving local selector: \"/tmp/ledger/local\""
    ));
    assert_eq!(err.code, ServerErrorCode::StoragePersistFailed);
}

#[test]
fn classifies_unmapped_error_as_request_failed() {
    let err = map_repo_scope_error(anyhow::anyhow!("something completely unexpected"));
    assert_eq!(err.code, ServerErrorCode::RequestFailed);
}

#[test]
fn classifies_url_coverage_validation_as_storage_persist_failed() {
    let err = map_repo_scope_error(anyhow::anyhow!(
        "Broken remote repo wiki while validating URL coverage: repository URL not resolved"
    ));
    assert_eq!(err.code, ServerErrorCode::StoragePersistFailed);
}
