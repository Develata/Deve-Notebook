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
    fn maps_repo_selector_mismatch_to_repo_context_invalid() {
        let err = map_repo_scope_error(anyhow::anyhow!(
            "Repo selector mismatch: repo_id resolved to default, repo_name resolved to test"
        ));
        assert_eq!(err.code, ServerErrorCode::ScRepoContextInvalid);
    }

    #[test]
    fn maps_scope_missing_repo_to_storage_not_found() {
        let err = map_repo_scope_error(anyhow::anyhow!("Repository not found: default"));
        assert_eq!(err.code, ServerErrorCode::StorageNotFound);
    }

    #[test]
    fn maps_scope_locked_db_to_storage_db_locked() {
        let err = map_repo_scope_error(anyhow::anyhow!(
            "Database already open. Cannot acquire lock."
        ));
        assert_eq!(err.code, ServerErrorCode::StorageDbLocked);
    }

    #[test]
    fn maps_scope_ambiguous_local_selector_to_repo_context_invalid() {
        let err =
            map_repo_scope_error(anyhow::anyhow!("Ambiguous local repository selector: wiki"));
        assert_eq!(err.code, ServerErrorCode::ScRepoContextInvalid);
    }

    #[test]
    fn maps_scope_remote_bootstrap_drift_to_repo_context_invalid() {
        let err = map_repo_scope_error(anyhow::anyhow!(
            "Cannot bootstrap local repo while on remote branch"
        ));
        assert_eq!(err.code, ServerErrorCode::ScRepoContextInvalid);
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
    fn maps_missing_selector_with_multiple_local_repos() {
        let err = map_repo_error(
            ScOp::ListChanges,
            anyhow::anyhow!("Active repository not selected: multiple local repos exist"),
        );
        assert_eq!(err.code, ServerErrorCode::ScRepoNotSelected);
    }

    #[test]
    fn maps_repo_error_selector_mismatch_to_repo_context_invalid() {
        let err = map_repo_error(
            ScOp::ListChanges,
            anyhow::anyhow!(
                "Repo selector mismatch: repo_id resolved to default, repo_name resolved to test"
            ),
        );
        assert_eq!(err.code, ServerErrorCode::ScRepoContextInvalid);
    }

    #[test]
    fn maps_repo_error_missing_repo_to_storage_not_found() {
        let err = map_repo_error(
            ScOp::ListChanges,
            anyhow::anyhow!("Repository not found: default"),
        );
        assert_eq!(err.code, ServerErrorCode::StorageNotFound);
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
}
