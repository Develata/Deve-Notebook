//! plan_ref:
//!   - 07_diff_logic#source-control-runtime

use super::super::{ProxyScOp, decode_error, decode_error_with_op};
use deve_core::protocol::ServerErrorCode;
use reqwest::StatusCode;

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
