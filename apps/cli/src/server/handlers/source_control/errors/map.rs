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
        ScOp::StagePending(path) if detail.contains("Path is not in pending_fs_ops") => Some(
            ServerError::with_detail(ServerErrorCode::ScPendingNotFound, path.clone()),
        ),
        ScOp::DiscardPending(path) if detail.contains("Path is not in pending_fs_ops") => Some(
            ServerError::with_detail(ServerErrorCode::ScPendingNotFound, path.clone()),
        ),
        ScOp::Unstage(path) if detail.contains("Path is not staged") => Some(
            ServerError::with_detail(ServerErrorCode::ScStagedNotFound, path.clone()),
        ),
        ScOp::DiffDoc(path)
            if contains_any(
                detail,
                &[
                    "Doc not found",
                    "Document not found",
                    "Remote document not found",
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
        ScOp::Commit if detail.to_ascii_lowercase().contains("nothing to commit") => {
            Some(ServerError::new(ServerErrorCode::ScNothingToCommit))
        }
        _ => None,
    }
}

fn classify_common_scope_code(detail: &str) -> Option<ServerErrorCode> {
    let lower = detail.to_ascii_lowercase();
    if contains_any(
        &lower,
        &[
            "active repository not selected",
            "multiple local repos exist",
            "no local repositories available",
        ],
    ) {
        return Some(ServerErrorCode::ScRepoNotSelected);
    }
    if contains_any(
        &lower,
        &[
            "repository not found:",
            "document not found",
            "doc not found",
        ],
    ) {
        return Some(ServerErrorCode::StorageNotFound);
    }
    if contains_any(
        &lower,
        &[
            "database already open",
            "cannot acquire lock",
            "db locked",
            "database is locked",
            "failed to lock database",
        ],
    ) {
        return Some(ServerErrorCode::StorageDbLocked);
    }
    if contains_any(
        &lower,
        &[
            "broken local repo",
            "broken shadow repo",
            "broken shadow peer",
            "failed to walk local repo",
            "deserialize",
            "decode",
            "unexpected end",
        ],
    ) {
        return Some(ServerErrorCode::StoragePersistFailed);
    }
    if lower.contains("tracked document projection missing") {
        return Some(ServerErrorCode::StoragePersistFailed);
    }
    if contains_any(
        &lower,
        &[
            "remote session lost repo name",
            "cannot bootstrap local repo while on remote branch",
            "repository uuid not resolved",
            "remote repository selector not resolved",
            "local repository selector not resolved",
            "local repository uuid not resolved",
            "session repo mismatch",
            "repo selector mismatch",
            "ambiguous local repository selector",
            "ambiguous remote repository selector",
            "local repo not found for uuid",
            "local repo operation requested on remote branch",
            "local workspace path requested on remote branch",
            "local workspace root requested on remote branch",
        ],
    ) {
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
