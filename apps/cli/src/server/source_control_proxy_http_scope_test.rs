//! plan_ref:
//!   - 07_diff_logic#source-control-runtime

use super::super::decode_error;
use deve_core::protocol::ServerErrorCode;
use reqwest::StatusCode;

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
