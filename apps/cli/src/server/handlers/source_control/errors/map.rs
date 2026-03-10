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
    ResolveConflict(String),
}

pub fn map_repo_scope_error(error: Error) -> ServerError {
    let detail = error.to_string();
    let lower = detail.to_ascii_lowercase();
    if lower.contains("active repository not selected") {
        return ServerError::with_detail(ServerErrorCode::ScRepoNotSelected, detail);
    }
    if contains_any(
        &lower,
        &[
            "remote session lost repo name",
            "repository uuid not resolved",
            "session repo mismatch",
        ],
    ) {
        return ServerError::with_detail(ServerErrorCode::ScRepoContextInvalid, detail);
    }
    ServerError::with_detail(ServerErrorCode::RequestFailed, detail)
}

pub fn map_repo_error(op: ScOp, error: Error) -> ServerError {
    let detail = error.to_string();
    if let Ok(error) = serde_json::from_str::<ServerError>(&detail) {
        return error;
    }
    if contains_any(
        &detail.to_ascii_lowercase(),
        &["database is locked", "failed to lock database"],
    ) {
        return ServerError::with_detail(ServerErrorCode::StorageDbLocked, detail);
    }
    if detail.to_ascii_lowercase().contains("conflict") {
        return ServerError::with_detail(ServerErrorCode::StorageConflict, detail);
    }
    match op {
        ScOp::StagePending(path) if detail.contains("Path is not in pending_fs_ops") => {
            ServerError::with_detail(ServerErrorCode::ScPendingNotFound, path)
        }
        ScOp::DiscardPending(path) if detail.contains("Path is not in pending_fs_ops") => {
            ServerError::with_detail(ServerErrorCode::ScPendingNotFound, path)
        }
        ScOp::Unstage(path) if detail.contains("Path is not staged") => {
            ServerError::with_detail(ServerErrorCode::ScStagedNotFound, path)
        }
        ScOp::DiffDoc(path)
            if contains_any(
                &detail,
                &[
                    "Doc not found",
                    "Document not found",
                    "Remote document not found",
                ],
            ) =>
        {
            ServerError::with_detail(ServerErrorCode::ScDocNotFound, path)
        }
        ScOp::CommitDiff(commit_id) if detail.contains("Commit not found") => {
            ServerError::with_detail(ServerErrorCode::ScCommitNotFound, commit_id)
        }
        ScOp::Commit if detail.to_ascii_lowercase().contains("nothing to commit") => {
            ServerError::new(ServerErrorCode::ScNothingToCommit)
        }
        ScOp::ResolveConflict(path) if detail.contains("Pending change not found") => {
            ServerError::with_detail(ServerErrorCode::ScConflictTargetMissing, path)
        }
        ScOp::ListPending
        | ScOp::ListChanges
        | ScOp::CommitHistory
        | ScOp::StagePending(_)
        | ScOp::DiscardPending(_)
        | ScOp::Unstage(_)
        | ScOp::DiffDoc(_)
        | ScOp::CommitDiff(_)
        | ScOp::Commit
        | ScOp::ResolveConflict(_) => {
            ServerError::with_detail(ServerErrorCode::RequestFailed, detail)
        }
    }
}

fn contains_any(input: &str, patterns: &[&str]) -> bool {
    patterns.iter().any(|pattern| input.contains(pattern))
}
#[cfg(test)]
mod tests {
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
    fn maps_repo_scope_miss_to_repo_not_selected() {
        let err = map_repo_scope_error(anyhow::anyhow!(
            "Active repository not selected for current session"
        ));
        assert_eq!(err.code, ServerErrorCode::ScRepoNotSelected);
    }

    #[test]
    fn preserves_remote_structured_errors() {
        let err = map_repo_error(
            ScOp::Commit,
            anyhow::anyhow!("{}", r#"{"code":"SC_NOTHING_TO_COMMIT"}"#),
        );
        assert_eq!(err.code, ServerErrorCode::ScNothingToCommit);
    }
}
