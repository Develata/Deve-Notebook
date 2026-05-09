//! plan_ref:
//!   - 06_repository#repo-scope-runtime
//!
//! Operation-specific source-control error mapping tests.

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
fn preserves_remote_structured_errors() {
    let err = map_repo_error(
        ScOp::Commit,
        anyhow::anyhow!("{}", r#"{"code":"SC_NOTHING_TO_COMMIT"}"#),
    );
    assert_eq!(err.code, ServerErrorCode::ScNothingToCommit);
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

#[test]
fn maps_stage_target_resolution_miss_to_pending_not_found() {
    let err = map_repo_error(
        ScOp::StagePending("notes/a.md".into()),
        anyhow::anyhow!("Source control target not resolved for path notes/a.md"),
    );
    assert_eq!(err.code, ServerErrorCode::ScPendingNotFound);
    assert_eq!(err.detail.as_deref(), Some("notes/a.md"));
}

#[test]
fn maps_unstage_identity_guard_to_storage_conflict() {
    let err = map_repo_error(
        ScOp::Unstage("notes/a.md".into()),
        anyhow::anyhow!("Tracked source control target requires document identity: notes/a.md"),
    );
    assert_eq!(err.code, ServerErrorCode::StorageConflict);
    assert_eq!(err.detail.as_deref(), Some("notes/a.md"));
}

#[test]
fn maps_diff_legacy_projection_gap_to_storage_persist_failed() {
    let err = map_repo_error(
        ScOp::DiffDoc("notes/a.md".into()),
        anyhow::anyhow!("Tracked document projection missing for legacy-mapped path: notes/a.md"),
    );
    assert_eq!(err.code, ServerErrorCode::StoragePersistFailed);
}

#[test]
fn maps_commit_diff_projection_gap_to_sc_commit_diff_unprojectable() {
    let err = map_repo_error(
        ScOp::CommitDiff("387cc45d".into()),
        anyhow::anyhow!(
            "Commit diff lost projected path for doc 310d949b-6a72-4ea5-91e4-4dab77b9ef3e between seq 2590 and 2828"
        ),
    );
    assert_eq!(err.code, ServerErrorCode::ScCommitDiffUnprojectable);
    assert_eq!(err.detail.as_deref(), Some("387cc45d"));
}
