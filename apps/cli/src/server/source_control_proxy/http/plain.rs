//! plan_ref:
//!   - 05_diff_logic#source-control-runtime

use crate::server::error_classify::{
    is_db_locked, is_repo_context_invalid, is_repo_not_selected, is_stale_scope,
    is_storage_corruption,
};
use deve_core::protocol::{ServerError, ServerErrorCode};
use reqwest::StatusCode;

use super::target::{ProxyScOp, classify_op_specific_error};

pub(super) fn decode_plain_text_error(
    status: StatusCode,
    raw_detail: &str,
    op: Option<&ProxyScOp>,
) -> ServerError {
    let lower = raw_detail.to_ascii_lowercase();
    if let Some(error) = classify_op_specific_error(op, &lower) {
        return error;
    }
    if is_repo_not_selected(&lower) {
        return ServerError::with_detail(ServerErrorCode::ScRepoNotSelected, raw_detail);
    }
    if is_stale_scope(&lower) {
        return ServerError::with_detail(ServerErrorCode::ScStaleScope, raw_detail);
    }
    if is_storage_corruption(&lower) {
        return ServerError::with_detail(ServerErrorCode::StoragePersistFailed, raw_detail);
    }
    if is_repo_context_invalid(&lower) {
        return ServerError::with_detail(ServerErrorCode::ScRepoContextInvalid, raw_detail);
    }
    if is_db_locked(&lower) || status == StatusCode::SERVICE_UNAVAILABLE {
        return ServerError::with_detail(
            ServerErrorCode::StorageDbLocked,
            format_remote_detail(status, raw_detail),
        );
    }
    decode_source_control_detail(status, raw_detail, &lower)
}

fn decode_source_control_detail(status: StatusCode, raw_detail: &str, lower: &str) -> ServerError {
    if lower.contains("path is not in pending_fs_ops") {
        return ServerError::with_detail(ServerErrorCode::ScPendingNotFound, raw_detail);
    }
    if contains_any(
        lower,
        &["ambiguous pending_fs target", "ambiguous staged target"],
    ) {
        return ServerError::with_detail(ServerErrorCode::StorageConflict, raw_detail);
    }
    if lower.contains("path is not staged") {
        return ServerError::with_detail(ServerErrorCode::ScStagedNotFound, raw_detail);
    }
    if lower.contains("commit not found") {
        return ServerError::with_detail(ServerErrorCode::ScCommitNotFound, raw_detail);
    }
    if lower.contains("commit diff lost projected path for doc") {
        return ServerError::new(ServerErrorCode::ScCommitDiffUnprojectable);
    }
    if lower.contains("nothing to commit") {
        return ServerError::new(ServerErrorCode::ScNothingToCommit);
    }
    decode_doc_or_fallback(status, raw_detail, lower)
}

fn decode_doc_or_fallback(status: StatusCode, raw_detail: &str, lower: &str) -> ServerError {
    if contains_any(
        lower,
        &[
            "doc not found",
            "document not found",
            "remote document not found",
        ],
    ) {
        return ServerError::with_detail(ServerErrorCode::ScDocNotFound, raw_detail);
    }
    if lower.contains("local repo not found for name") {
        return ServerError::with_detail(ServerErrorCode::StorageNotFound, raw_detail);
    }
    if lower.contains("conflict") {
        return ServerError::with_detail(ServerErrorCode::StorageConflict, raw_detail);
    }
    if status == StatusCode::NOT_FOUND {
        return ServerError::with_detail(
            ServerErrorCode::StorageNotFound,
            format_remote_detail(status, raw_detail),
        );
    }
    ServerError::with_detail(
        ServerErrorCode::RequestFailed,
        format_remote_detail(status, raw_detail),
    )
}

fn format_remote_detail(status: StatusCode, raw_detail: &str) -> String {
    if raw_detail.is_empty() {
        format!("remote source control request failed with HTTP {status}")
    } else {
        format!("remote source control request failed with HTTP {status}: {raw_detail}")
    }
}

fn contains_any(input: &str, patterns: &[&str]) -> bool {
    patterns.iter().any(|pattern| input.contains(pattern))
}
