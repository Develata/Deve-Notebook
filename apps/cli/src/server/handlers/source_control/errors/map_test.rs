use super::*;

#[test]
fn maps_pending_miss_with_operation_context() {
    let err = map_repo_error(
        ScOp::StagePending("notes/a.md".into()),
        anyhow::anyhow!("Path is not in pending_fs_ops: notes/a.md"),
    );
    assert_eq!(err.code, ServerErrorCode::ScPendingNotFound);
    assert_eq!(err.detail.as_deref(), Some("notes/a.md"));
}

#[test]
fn maps_pending_ambiguity_to_storage_conflict() {
    let err = map_repo_error(
        ScOp::StagePending("notes/a.md".into()),
        anyhow::anyhow!("Ambiguous pending_fs target: notes/a.md matched tracked entries"),
    );
    assert_eq!(err.code, ServerErrorCode::StorageConflict);
    assert_eq!(err.detail.as_deref(), Some("notes/a.md"));
}

#[test]
fn maps_staged_ambiguity_to_storage_conflict() {
    let err = map_repo_error(
        ScOp::Unstage("notes/a.md".into()),
        anyhow::anyhow!("Ambiguous staged target: notes/a.md matched multiple live entries"),
    );
    assert_eq!(err.code, ServerErrorCode::StorageConflict);
    assert_eq!(err.detail.as_deref(), Some("notes/a.md"));
}

#[test]
fn maps_diff_ambiguity_to_storage_conflict() {
    let err = map_repo_error(
        ScOp::DiffDoc("notes/a.md".into()),
        anyhow::anyhow!("Ambiguous pending_fs target: notes/a.md matched tracked entries"),
    );
    assert_eq!(err.code, ServerErrorCode::StorageConflict);
    assert_eq!(err.detail.as_deref(), Some("notes/a.md"));
}

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
fn maps_scope_stale_nonce_to_repo_context_invalid() {
    let err = map_repo_scope_error(anyhow::anyhow!(
        "Browser SyncHello stale scope nonce: current_scope_nonce=9, requested_scope_nonce=7"
    ));
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
fn preserves_remote_structured_errors() {
    let err = map_repo_error(
        ScOp::Commit,
        anyhow::anyhow!("{}", r#"{"code":"SC_NOTHING_TO_COMMIT"}"#),
    );
    assert_eq!(err.code, ServerErrorCode::ScNothingToCommit);
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

#[test]
fn maps_diff_doc_missing_to_sc_doc_not_found_before_generic_storage_mapping() {
    let err = map_repo_error(
        ScOp::DiffDoc("notes/a.md".into()),
        anyhow::anyhow!("Document not found: notes/a.md"),
    );
    assert_eq!(err.code, ServerErrorCode::ScDocNotFound);
    assert_eq!(err.detail.as_deref(), Some("notes/a.md"));
}
