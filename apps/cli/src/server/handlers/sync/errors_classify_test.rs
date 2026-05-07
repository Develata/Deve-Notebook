//! plan_ref:
//!   - 05_network#server-ws-runtime

use super::classify_failure_code;
use deve_core::protocol::ServerErrorCode;

#[test]
fn classifies_signature_failures_as_peer_unauthenticated() {
    assert_eq!(
        classify_failure_code("Handshake failed: signature verification failed"),
        ServerErrorCode::SyncPeerUnauthenticated
    );
}

#[test]
fn classifies_snapshot_generation_as_storage_persist_failed() {
    assert_eq!(
        classify_failure_code("Failed to generate snapshot for repo x"),
        ServerErrorCode::StoragePersistFailed
    );
}

#[test]
fn classifies_missing_sync_scope_as_repo_unbound() {
    assert_eq!(
        classify_failure_code("Active repository not selected: multiple local repos exist"),
        ServerErrorCode::SyncRepoUnbound
    );
}

#[test]
fn classifies_sync_repo_scope_drift_as_repo_context_invalid() {
    assert_eq!(
        classify_failure_code(
            "Repo selector mismatch: repo_id resolved to default, repo_name resolved to test"
        ),
        ServerErrorCode::ScRepoContextInvalid
    );
}

#[test]
fn classifies_missing_sync_repo_as_storage_not_found() {
    assert_eq!(
        classify_failure_code("Repository not found: default"),
        ServerErrorCode::StorageNotFound
    );
}

#[test]
fn classifies_ambiguous_local_selector_as_repo_context_invalid() {
    assert_eq!(
        classify_failure_code("Ambiguous local repository selector: wiki"),
        ServerErrorCode::ScRepoContextInvalid
    );
}

#[test]
fn classifies_remote_bootstrap_drift_as_repo_context_invalid() {
    assert_eq!(
        classify_failure_code("Cannot bootstrap local repo while on remote branch"),
        ServerErrorCode::ScRepoContextInvalid
    );
}

#[test]
fn classifies_stale_scope_as_stale_scope() {
    assert_eq!(
        classify_failure_code(
            "Browser SyncHello stale scope nonce: current_scope_nonce=9, requested_scope_nonce=7"
        ),
        ServerErrorCode::ScStaleScope
    );
}

#[test]
fn classifies_sync_repo_mismatch_as_route_mismatch() {
    assert_eq!(
        classify_failure_code("Sync repo mismatch: session bound to None, got repo-a"),
        ServerErrorCode::SyncRepoRouteMismatch
    );
}

#[test]
fn classifies_invalid_sync_payload_as_sync_invalid_payload() {
    assert_eq!(
        classify_failure_code("Encrypted op seq mismatch: envelope 1, payload 2"),
        ServerErrorCode::SyncInvalidPayload
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

#[test]
fn classifies_broken_repo_entry_as_storage_persist_failed() {
    assert_eq!(
        classify_failure_code(
            "Broken repo entry \"/tmp/local/.redb\" while listing repos: invalid file stem"
        ),
        ServerErrorCode::StoragePersistFailed
    );
}

#[test]
fn classifies_missing_remote_catalog_as_storage_persist_failed() {
    assert_eq!(
        classify_failure_code(
            "Broken remote repo catalog: remote repo directory missing at /tmp/ledger/remotes"
        ),
        ServerErrorCode::StoragePersistFailed
    );
}
