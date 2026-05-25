//! plan_ref:
//!   - 05_diff_logic#source-control-runtime

use super::super::decode_error;
use deve_core::protocol::ServerErrorCode;
use reqwest::StatusCode;

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
