use crate::server::error_classify::{
    is_db_locked, is_repo_context_invalid, is_repo_not_selected, is_storage_corruption,
    is_storage_not_found,
};
use anyhow::Error;
use deve_core::protocol::{ServerError, ServerErrorCode};

pub enum ScOp {
    ListPending,
    ListChanges,
    StagePending(String),
    DiscardPending(String),
    Unstage(String),
    DiffDoc(String),
    CommitHistory,
    CommitDiff(String),
    Commit,
}

pub fn map_repo_scope_error(error: Error) -> ServerError {
    let detail = error.to_string();
    if let Some(code) = classify_common_scope_code(&detail) {
        return ServerError::with_detail(code, detail);
    }
    ServerError::with_detail(ServerErrorCode::RequestFailed, detail)
}

pub fn map_repo_error(op: ScOp, error: Error) -> ServerError {
    let detail = error.to_string();
    if let Ok(error) = serde_json::from_str::<ServerError>(&detail) {
        return error;
    }
    if let Some(error) = classify_op_specific_error(&op, &detail) {
        return error;
    }
    if let Some(code) = classify_common_scope_code(&detail) {
        return ServerError::with_detail(code, detail);
    }
    if detail.to_ascii_lowercase().contains("conflict") {
        return ServerError::with_detail(ServerErrorCode::StorageConflict, detail);
    }
    ServerError::with_detail(ServerErrorCode::RequestFailed, detail)
}

fn classify_op_specific_error(op: &ScOp, detail: &str) -> Option<ServerError> {
    match op {
        ScOp::StagePending(path)
            if contains_any(
                detail,
                &[
                    "Ambiguous pending_fs target",
                    "Tracked source control target requires document identity",
                ],
            ) =>
        {
            Some(ServerError::with_detail(
                ServerErrorCode::StorageConflict,
                path.clone(),
            ))
        }
        ScOp::StagePending(path)
            if contains_any(
                detail,
                &[
                    "Path is not in pending_fs_ops",
                    "Source control target not resolved for path",
                    "Source control target not resolved for doc",
                ],
            ) =>
        {
            Some(ServerError::with_detail(
                ServerErrorCode::ScPendingNotFound,
                path.clone(),
            ))
        }
        ScOp::DiscardPending(path)
            if contains_any(
                detail,
                &[
                    "Ambiguous pending_fs target",
                    "Tracked source control target requires document identity",
                ],
            ) =>
        {
            Some(ServerError::with_detail(
                ServerErrorCode::StorageConflict,
                path.clone(),
            ))
        }
        ScOp::DiscardPending(path)
            if contains_any(
                detail,
                &[
                    "Path is not in pending_fs_ops",
                    "Source control target not resolved for path",
                    "Source control target not resolved for doc",
                ],
            ) =>
        {
            Some(ServerError::with_detail(
                ServerErrorCode::ScPendingNotFound,
                path.clone(),
            ))
        }
        ScOp::Unstage(path)
            if contains_any(
                detail,
                &[
                    "Ambiguous staged target",
                    "Tracked source control target requires document identity",
                ],
            ) =>
        {
            Some(ServerError::with_detail(
                ServerErrorCode::StorageConflict,
                path.clone(),
            ))
        }
        ScOp::Unstage(path)
            if contains_any(
                detail,
                &[
                    "Path is not staged",
                    "Source control target not resolved for path",
                    "Source control target not resolved for doc",
                ],
            ) =>
        {
            Some(ServerError::with_detail(
                ServerErrorCode::ScStagedNotFound,
                path.clone(),
            ))
        }
        ScOp::DiffDoc(path)
            if contains_any(
                detail,
                &[
                    "Ambiguous pending_fs target",
                    "Ambiguous staged target",
                    "Tracked source control target requires document identity",
                ],
            ) =>
        {
            Some(ServerError::with_detail(
                ServerErrorCode::StorageConflict,
                path.clone(),
            ))
        }
        ScOp::DiffDoc(path)
            if contains_any(
                detail,
                &[
                    "Doc not found",
                    "Document not found",
                    "Remote document not found",
                    "Source control target not resolved for path",
                    "Source control target not resolved for doc",
                ],
            ) =>
        {
            Some(ServerError::with_detail(
                ServerErrorCode::ScDocNotFound,
                path.clone(),
            ))
        }
        ScOp::CommitDiff(commit_id) if detail.contains("Commit not found") => Some(
            ServerError::with_detail(ServerErrorCode::ScCommitNotFound, commit_id.clone()),
        ),
        ScOp::CommitDiff(commit_id) if detail.contains("Commit diff lost projected path for doc") => {
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

fn classify_common_scope_code(detail: &str) -> Option<ServerErrorCode> {
    let lower = detail.to_ascii_lowercase();
    if is_repo_not_selected(&lower) {
        return Some(ServerErrorCode::ScRepoNotSelected);
    }
    if is_storage_not_found(&lower) {
        return Some(ServerErrorCode::StorageNotFound);
    }
    if is_db_locked(&lower) {
        return Some(ServerErrorCode::StorageDbLocked);
    }
    if is_storage_corruption(&lower) {
        return Some(ServerErrorCode::StoragePersistFailed);
    }
    if lower.contains("tracked document projection missing") {
        return Some(ServerErrorCode::StoragePersistFailed);
    }
    if is_repo_context_invalid(&lower) {
        return Some(ServerErrorCode::ScRepoContextInvalid);
    }
    None
}

fn contains_any(input: &str, patterns: &[&str]) -> bool {
    patterns.iter().any(|pattern| input.contains(pattern))
}

#[cfg(test)]
#[path = "map_test.rs"]
mod tests;
