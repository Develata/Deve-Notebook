use super::*;

#[test]
fn preserves_server_error_json() {
    let err = decode_error(
        StatusCode::BAD_REQUEST,
        br#"{"code":"SC_PENDING_NOT_FOUND","detail":"missing"}"#,
    );
    assert_eq!(err.code, ServerErrorCode::ScPendingNotFound);
    assert_eq!(err.detail.as_deref(), Some("missing"));
}

#[test]
fn wraps_plain_text_errors() {
    let err = decode_error(StatusCode::NOT_FOUND, b"notes/a.md");
    assert_eq!(err.code, ServerErrorCode::StorageNotFound);
    assert!(
        err.detail
            .as_deref()
            .is_some_and(|detail| detail.contains("404 Not Found"))
    );
}

#[test]
fn maps_plain_text_pending_miss() {
    let err = decode_error(
        StatusCode::CONFLICT,
        b"Path is not in pending_fs_ops: notes/a.md",
    );
    assert_eq!(err.code, ServerErrorCode::ScPendingNotFound);
    assert_eq!(
        err.detail.as_deref(),
        Some("Path is not in pending_fs_ops: notes/a.md")
    );
}

#[test]
fn maps_plain_text_ambiguous_target_to_storage_conflict() {
    let err = decode_error(
        StatusCode::CONFLICT,
        b"Ambiguous pending_fs target: notes/a.md matched tracked entries",
    );
    assert_eq!(err.code, ServerErrorCode::StorageConflict);
    assert_eq!(
        err.detail.as_deref(),
        Some("Ambiguous pending_fs target: notes/a.md matched tracked entries")
    );
}

#[test]
fn maps_plain_text_repo_scope_drift() {
    let err = decode_error(
        StatusCode::CONFLICT,
        b"Repo selector mismatch: repo_id resolved to default, repo_name resolved to test",
    );
    assert_eq!(err.code, ServerErrorCode::ScRepoContextInvalid);
}

#[test]
fn maps_missing_remote_catalog_as_storage_persist_failed() {
    let err = decode_error(
        StatusCode::INTERNAL_SERVER_ERROR,
        b"Broken remote repo catalog: remote repo directory missing at /tmp/ledger/remotes",
    );
    assert_eq!(err.code, ServerErrorCode::StoragePersistFailed);
}

#[test]
fn maps_plain_text_remote_bootstrap_drift() {
    let err = decode_error(
        StatusCode::CONFLICT,
        b"Cannot bootstrap local repo while on remote branch",
    );
    assert_eq!(err.code, ServerErrorCode::ScRepoContextInvalid);
}

#[test]
fn maps_plain_text_broken_repo_entry_to_storage_persist_failed() {
    let err = decode_error(
        StatusCode::INTERNAL_SERVER_ERROR,
        br#"Broken repo entry "/tmp/local/.redb" while listing repos: invalid file stem"#,
    );
    assert_eq!(err.code, ServerErrorCode::StoragePersistFailed);
}

#[test]
fn maps_plain_text_stale_scope_nonce() {
    let err = decode_error(
        StatusCode::CONFLICT,
        b"Browser SyncHello stale scope nonce: current_scope_nonce=9, requested_scope_nonce=7",
    );
    assert_eq!(err.code, ServerErrorCode::ScRepoContextInvalid);
}

#[test]
fn maps_plain_text_legacy_projection_breakage() {
    let err = decode_error(
        StatusCode::CONFLICT,
        b"Tracked document projection missing for legacy-mapped path: notes/legacy.md",
    );
    assert_eq!(err.code, ServerErrorCode::StoragePersistFailed);
    assert_eq!(
        err.detail.as_deref(),
        Some("Tracked document projection missing for legacy-mapped path: notes/legacy.md")
    );
}

#[test]
fn maps_plain_text_missing_local_repo_name() {
    let err = decode_error(StatusCode::NOT_FOUND, b"Local repo not found for name wiki");
    assert_eq!(err.code, ServerErrorCode::StorageNotFound);
    assert_eq!(
        err.detail.as_deref(),
        Some("Local repo not found for name wiki")
    );
}

#[test]
fn maps_target_identity_guard_with_operation_context() {
    let err = decode_error_with_op(
        StatusCode::CONFLICT,
        b"Tracked source control target requires document identity: notes/a.md",
        Some(&ProxyScOp::Unstage("notes/a.md".into())),
    );
    assert_eq!(err.code, ServerErrorCode::StorageConflict);
    assert_eq!(err.detail.as_deref(), Some("notes/a.md"));
}

#[test]
fn maps_target_resolution_miss_with_stage_context() {
    let err = decode_error_with_op(
        StatusCode::CONFLICT,
        b"Source control target not resolved for path notes/a.md",
        Some(&ProxyScOp::StagePending("notes/a.md".into())),
    );
    assert_eq!(err.code, ServerErrorCode::ScPendingNotFound);
    assert_eq!(err.detail.as_deref(), Some("notes/a.md"));
}

#[test]
fn maps_target_resolution_miss_with_diff_context() {
    let err = decode_error_with_op(
        StatusCode::CONFLICT,
        b"Source control target not resolved for doc 00000000-0000-0000-0000-000000000000 at notes/a.md",
        Some(&ProxyScOp::DiffDoc("notes/a.md".into())),
    );
    assert_eq!(err.code, ServerErrorCode::ScDocNotFound);
    assert_eq!(err.detail.as_deref(), Some("notes/a.md"));
}

#[test]
fn maps_plain_text_path_not_staged() {
    let err = decode_error(StatusCode::CONFLICT, b"Path is not staged: notes/a.md");
    assert_eq!(err.code, ServerErrorCode::ScStagedNotFound);
}

#[test]
fn maps_plain_text_commit_not_found() {
    let err = decode_error(StatusCode::NOT_FOUND, b"commit not found for sequence 42");
    assert_eq!(err.code, ServerErrorCode::ScCommitNotFound);
}

#[test]
fn maps_plain_text_commit_diff_projection_gap() {
    let err = decode_error(
        StatusCode::CONFLICT,
        b"Commit diff lost projected path for doc 310d949b-6a72-4ea5-91e4-4dab77b9ef3e between seq 2590 and 2828",
    );
    assert_eq!(err.code, ServerErrorCode::ScCommitDiffUnprojectable);
    assert_eq!(err.detail, None);
}

#[test]
fn maps_plain_text_nothing_to_commit() {
    let err = decode_error(StatusCode::CONFLICT, b"nothing to commit");
    assert_eq!(err.code, ServerErrorCode::ScNothingToCommit);
}

#[test]
fn maps_plain_text_doc_not_found() {
    let err = decode_error(StatusCode::NOT_FOUND, b"doc not found: notes/a.md");
    assert_eq!(err.code, ServerErrorCode::ScDocNotFound);
}

#[test]
fn maps_plain_text_generic_conflict_fallback() {
    let err = decode_error(StatusCode::CONFLICT, b"some unknown conflict scenario");
    assert_eq!(err.code, ServerErrorCode::StorageConflict);
}

#[test]
fn maps_db_locked_via_status_503() {
    let err = decode_error(StatusCode::SERVICE_UNAVAILABLE, b"server overloaded");
    assert_eq!(err.code, ServerErrorCode::StorageDbLocked);
}

#[test]
fn maps_unmapped_error_as_request_failed() {
    let err = decode_error(
        StatusCode::INTERNAL_SERVER_ERROR,
        b"something completely unexpected",
    );
    assert_eq!(err.code, ServerErrorCode::RequestFailed);
}
