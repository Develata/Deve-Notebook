use super::classify_listing_error;
use deve_core::protocol::ServerErrorCode;

#[test]
fn classifies_locked_listing_db_as_storage_db_locked() {
    assert_eq!(
        classify_listing_error("Database already open. Cannot acquire lock."),
        ServerErrorCode::StorageDbLocked
    );
}

#[test]
fn classifies_listing_scope_drift_as_repo_context_invalid() {
    assert_eq!(
        classify_listing_error("Remote session lost repo name for current branch"),
        ServerErrorCode::ScRepoContextInvalid
    );
}

#[test]
fn classifies_ambiguous_remote_selector_as_repo_context_invalid() {
    assert_eq!(
        classify_listing_error("Ambiguous remote repository selector: shadow-wiki"),
        ServerErrorCode::ScRepoContextInvalid
    );
}

#[test]
fn classifies_missing_repo_selection_as_sync_repo_unbound() {
    assert_eq!(
        classify_listing_error("Active repository not selected: multiple local repos exist"),
        ServerErrorCode::SyncRepoUnbound
    );
}

#[test]
fn classifies_remote_workspace_access_as_repo_context_invalid() {
    assert_eq!(
        classify_listing_error("Local workspace root requested on remote branch: wiki"),
        ServerErrorCode::ScRepoContextInvalid
    );
}

#[test]
fn classifies_broken_shadow_listing_as_storage_persist_failed() {
    assert_eq!(
        classify_listing_error("Broken shadow repo notes for peer peer-a while listing repos"),
        ServerErrorCode::StoragePersistFailed
    );
}
