//! plan_ref:
//!   - 03_rendering#document-authority-bridge
//!
//! Document error mapping regression coverage.

use super::error_code;
use deve_core::protocol::ServerErrorCode;

#[test]
fn classifies_missing_docs_as_doc_not_found_and_repos_as_storage_not_found() {
    assert_eq!(
        error_code(&anyhow::anyhow!("Document not found: abc")),
        ServerErrorCode::DocNotFound
    );
    assert_eq!(
        error_code(&anyhow::anyhow!("Repository not found: wiki")),
        ServerErrorCode::StorageNotFound
    );
}

#[test]
fn classifies_document_context_invalid_as_doc_context_invalid() {
    assert_eq!(
        error_code(&anyhow::anyhow!("Document context invalid: stale doc_id")),
        ServerErrorCode::DocContextInvalid
    );
}

#[test]
fn classifies_locked_databases_as_storage_db_locked() {
    assert_eq!(
        error_code(&anyhow::anyhow!(
            "Database already open. Cannot acquire lock."
        )),
        ServerErrorCode::StorageDbLocked
    );
}

#[test]
fn classifies_repo_scope_drift_as_repo_context_invalid() {
    assert_eq!(
        error_code(&anyhow::anyhow!(
            "Repo selector mismatch: repo_id resolved to default, repo_name resolved to test"
        )),
        ServerErrorCode::ScRepoContextInvalid
    );
}

#[test]
fn classifies_missing_repo_selection_as_sync_repo_unbound() {
    assert_eq!(
        error_code(&anyhow::anyhow!(
            "Active repository not selected: multiple local repos exist"
        )),
        ServerErrorCode::SyncRepoUnbound
    );
}

#[test]
fn classifies_legacy_projection_breakage_as_storage_persist_failed() {
    assert_eq!(
        error_code(&anyhow::anyhow!(
            "Tracked document projection missing for legacy-mapped path: notes/legacy.md"
        )),
        ServerErrorCode::StoragePersistFailed
    );
}

#[test]
fn classifies_missing_snapshot_tables_as_storage_persist_failed() {
    assert_eq!(
        error_code(&anyhow::anyhow!("Table 'snapshot_index' does not exist")),
        ServerErrorCode::StoragePersistFailed
    );
}
