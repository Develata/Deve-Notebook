use super::{
    classify_graph_projection_error, classify_repo_error, parse_doc_id, plugin_host_error_response,
};
use axum::http::StatusCode;
use deve_core::protocol::ServerErrorCode;

#[test]
fn invalid_doc_id_returns_small_error_detail() {
    assert_eq!(
        parse_doc_id("not-a-uuid").expect_err("must reject"),
        "invalid doc_id"
    );
}

#[test]
fn classifies_repo_scope_drift_as_conflict() {
    assert_eq!(
        classify_repo_error(
            "Repo selector mismatch: repo_id resolved to default, repo_name resolved to test"
        ),
        (StatusCode::CONFLICT, ServerErrorCode::ScRepoContextInvalid)
    );
}

#[test]
fn classifies_locked_repo_db_as_service_unavailable() {
    assert_eq!(
        classify_repo_error("Database already open. Cannot acquire lock."),
        (
            StatusCode::SERVICE_UNAVAILABLE,
            ServerErrorCode::StorageDbLocked
        )
    );
}

#[test]
fn classifies_missing_repo_selection_as_sync_repo_unbound() {
    assert_eq!(
        classify_repo_error("Active repository not selected: multiple local repos exist"),
        (StatusCode::CONFLICT, ServerErrorCode::SyncRepoUnbound)
    );
}

#[test]
fn classifies_stale_scope_nonce_as_repo_context_invalid() {
    assert_eq!(
        classify_repo_error(
            "Browser SyncHello stale scope nonce: current_scope_nonce=9, requested_scope_nonce=7"
        ),
        (StatusCode::CONFLICT, ServerErrorCode::ScRepoContextInvalid)
    );
}

#[test]
fn classifies_ambiguous_local_selector_as_repo_context_invalid() {
    assert_eq!(
        classify_repo_error("Ambiguous local repository selector: wiki"),
        (StatusCode::CONFLICT, ServerErrorCode::ScRepoContextInvalid)
    );
}

#[test]
fn classifies_remote_bootstrap_drift_as_repo_context_invalid() {
    assert_eq!(
        classify_repo_error("Cannot bootstrap local repo while on remote branch"),
        (StatusCode::CONFLICT, ServerErrorCode::ScRepoContextInvalid)
    );
}

#[test]
fn classifies_missing_local_repo_name_as_not_found() {
    assert_eq!(
        classify_repo_error("Local repo not found for name wiki"),
        (StatusCode::NOT_FOUND, ServerErrorCode::StorageNotFound)
    );
}

#[test]
fn graph_projection_degraded_error_maps_to_structured_code() {
    let error: anyhow::Error =
        crate::graph_projection::GraphProjectionError::DegradedProjectionRequired {
            repo_name: "default".into(),
            detail: "missing_parent: orphan".into(),
        }
        .into();

    assert_eq!(
        classify_graph_projection_error(&error, &error.to_string()),
        (
            StatusCode::CONFLICT,
            ServerErrorCode::GraphDegradedProjectionRequired
        )
    );
}

#[test]
fn classifies_broken_repo_entry_as_storage_persist_failed() {
    assert_eq!(
        classify_repo_error(
            "Broken repo entry \"/tmp/local/.redb\" while listing repos: invalid file stem"
        ),
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            ServerErrorCode::StoragePersistFailed
        )
    );
}

#[test]
fn classifies_missing_remote_catalog_as_storage_persist_failed() {
    assert_eq!(
        classify_repo_error(
            "Broken remote repo catalog: remote repo directory missing at /tmp/ledger/remotes"
        ),
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            ServerErrorCode::StoragePersistFailed
        )
    );
}

#[test]
fn plugin_host_repository_unavailable_maps_to_not_implemented() {
    let response = plugin_host_error_response("Repository not configured");
    assert_eq!(response.status(), StatusCode::NOT_IMPLEMENTED);
}
