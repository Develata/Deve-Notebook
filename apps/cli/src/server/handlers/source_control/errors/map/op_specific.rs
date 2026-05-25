//! plan_ref:
//!   - 04_repository#repo-scope-runtime
//!
//! Operation-specific source-control error classification.

use super::ScOp;
use deve_core::protocol::{ServerError, ServerErrorCode};

pub(super) fn classify_op_specific_error(op: &ScOp, detail: &str) -> Option<ServerError> {
    match op {
        ScOp::StagePending(path) if is_pending_conflict(detail) => Some(storage_conflict(path)),
        ScOp::StagePending(path) if is_pending_missing(detail) => Some(pending_not_found(path)),
        ScOp::DiscardPending(path) if is_pending_conflict(detail) => Some(storage_conflict(path)),
        ScOp::DiscardPending(path) if is_pending_missing(detail) => Some(pending_not_found(path)),
        ScOp::Unstage(path) if is_staged_conflict(detail) => Some(storage_conflict(path)),
        ScOp::Unstage(path) if is_staged_missing(detail) => Some(staged_not_found(path)),
        ScOp::DiffDoc(path) if is_diff_conflict(detail) => Some(storage_conflict(path)),
        ScOp::DiffDoc(path) if is_doc_missing(detail) => Some(doc_not_found(path)),
        ScOp::CommitDiff(commit_id) if detail.contains("Commit not found") => Some(
            ServerError::with_detail(ServerErrorCode::ScCommitNotFound, commit_id.clone()),
        ),
        ScOp::CommitDiff(commit_id)
            if detail.contains("Commit diff lost projected path for doc") =>
        {
            Some(ServerError::with_detail(
                ServerErrorCode::ScCommitDiffUnprojectable,
                commit_id.clone(),
            ))
        }
        ScOp::Commit if detail.to_ascii_lowercase().contains("nothing to commit") => {
            Some(ServerError::new(ServerErrorCode::ScNothingToCommit))
        }
        _ => None,
    }
}

fn is_pending_conflict(detail: &str) -> bool {
    contains_any(
        detail,
        &[
            "Ambiguous pending_fs target",
            "Tracked source control target requires document identity",
        ],
    )
}

fn is_pending_missing(detail: &str) -> bool {
    contains_any(
        detail,
        &[
            "Path is not in pending_fs_ops",
            "Source control target not resolved for path",
            "Source control target not resolved for doc",
        ],
    )
}

fn is_staged_conflict(detail: &str) -> bool {
    contains_any(
        detail,
        &[
            "Ambiguous staged target",
            "Tracked source control target requires document identity",
        ],
    )
}

fn is_staged_missing(detail: &str) -> bool {
    contains_any(
        detail,
        &[
            "Path is not staged",
            "Source control target not resolved for path",
            "Source control target not resolved for doc",
        ],
    )
}

fn is_diff_conflict(detail: &str) -> bool {
    is_pending_conflict(detail) || is_staged_conflict(detail)
}

fn is_doc_missing(detail: &str) -> bool {
    contains_any(
        detail,
        &[
            "Doc not found",
            "Document not found",
            "Remote document not found",
            "Source control target not resolved for path",
            "Source control target not resolved for doc",
        ],
    )
}

fn storage_conflict(path: &str) -> ServerError {
    ServerError::with_detail(ServerErrorCode::StorageConflict, path)
}

fn pending_not_found(path: &str) -> ServerError {
    ServerError::with_detail(ServerErrorCode::ScPendingNotFound, path)
}

fn staged_not_found(path: &str) -> ServerError {
    ServerError::with_detail(ServerErrorCode::ScStagedNotFound, path)
}

fn doc_not_found(path: &str) -> ServerError {
    ServerError::with_detail(ServerErrorCode::ScDocNotFound, path)
}

fn contains_any(input: &str, patterns: &[&str]) -> bool {
    patterns.iter().any(|pattern| input.contains(pattern))
}
