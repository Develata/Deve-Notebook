//! plan_ref:
//!   - 06_repository#repo-scope-runtime

use super::classify_failure_code;
use deve_core::protocol::ServerErrorCode;

#[test]
fn classifies_node_meta_breakage_as_storage_persist_failed() {
    assert_eq!(
        classify_failure_code("Node meta missing for node abc"),
        ServerErrorCode::StoragePersistFailed
    );
}

#[test]
fn classifies_remote_workspace_access_as_repo_context_invalid() {
    assert_eq!(
        classify_failure_code("Local workspace path requested on remote branch: wiki"),
        ServerErrorCode::ScRepoContextInvalid
    );
}

#[test]
fn classifies_missing_repo_selection_as_sync_repo_unbound() {
    assert_eq!(
        classify_failure_code("Active repository not selected: multiple local repos exist"),
        ServerErrorCode::SyncRepoUnbound
    );
}

#[test]
fn classifies_missing_docs_as_storage_not_found() {
    assert_eq!(
        classify_failure_code("Document not found: abc"),
        ServerErrorCode::StorageNotFound
    );
    assert_eq!(
        classify_failure_code("Source not found: notes/a.md"),
        ServerErrorCode::StorageNotFound
    );
}

#[test]
fn classifies_existing_targets_as_storage_conflict() {
    assert_eq!(
        classify_failure_code("Target file already exists on disk: notes/a.md"),
        ServerErrorCode::StorageConflict
    );
}

#[test]
fn classifies_legacy_projection_breakage_as_storage_persist_failed() {
    assert_eq!(
        classify_failure_code(
            "Tracked document projection missing for legacy-mapped path: notes/legacy.md"
        ),
        ServerErrorCode::StoragePersistFailed
    );
}
