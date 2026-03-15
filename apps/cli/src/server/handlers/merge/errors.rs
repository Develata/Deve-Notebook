use crate::server::channel::DualChannel;
use deve_core::protocol::{ServerError, ServerErrorCode};

fn send(ch: &DualChannel, code: ServerErrorCode, detail: impl Into<String>) {
    ch.send_protocol_error(ServerError::with_detail(code, detail));
}

pub(super) fn request_failed(ch: &DualChannel, detail: impl Into<String>) {
    send(ch, ServerErrorCode::RequestFailed, detail);
}

pub(super) fn storage_conflict(ch: &DualChannel, detail: impl Into<String>) {
    send(ch, ServerErrorCode::StorageConflict, detail);
}

pub(super) fn storage_not_found(ch: &DualChannel, detail: impl Into<String>) {
    send(ch, ServerErrorCode::StorageNotFound, detail);
}

pub(super) fn storage_persist_failed(ch: &DualChannel, detail: impl Into<String>) {
    send(ch, ServerErrorCode::StoragePersistFailed, detail);
}

pub(super) fn classified_failure(ch: &DualChannel, detail: impl Into<String>) {
    let detail = detail.into();
    send(ch, classify_failure_code(&detail), detail);
}

fn classify_failure_code(detail: &str) -> ServerErrorCode {
    let lower = detail.to_ascii_lowercase();
    if contains_any(
        &lower,
        &[
            "repository not found:",
            "document not found",
            "doc not found",
            "no local repository matched the active remote repository",
        ],
    ) {
        return ServerErrorCode::StorageNotFound;
    }
    if lower.contains("database already open")
        || lower.contains("cannot acquire lock")
        || lower.contains("db locked")
        || lower.contains("database is locked")
    {
        return ServerErrorCode::StorageDbLocked;
    }
    if lower.contains("tracked document projection missing") {
        return ServerErrorCode::StoragePersistFailed;
    }
    if contains_any(
        &lower,
        &[
            "local repo operation requested on remote branch",
            "cannot bootstrap local repo while on remote branch",
            "repo selector mismatch",
            "session repo mismatch",
            "remote session lost repo name",
            "repository uuid not resolved",
            "remote repository selector not resolved",
            "local repository uuid not resolved",
            "ambiguous local repository selector",
            "ambiguous remote repository selector",
            "local repo not found for uuid",
        ],
    ) {
        return ServerErrorCode::ScRepoContextInvalid;
    }
    ServerErrorCode::RequestFailed
}

fn contains_any(input: &str, patterns: &[&str]) -> bool {
    patterns.iter().any(|pattern| input.contains(pattern))
}

#[cfg(test)]
mod tests {
    use super::classify_failure_code;
    use deve_core::protocol::ServerErrorCode;

    #[test]
    fn classifies_missing_local_merge_repo_as_storage_not_found() {
        assert_eq!(
            classify_failure_code("No local repository matched the active remote repository"),
            ServerErrorCode::StorageNotFound
        );
    }

    #[test]
    fn classifies_locked_merge_db_as_storage_db_locked() {
        assert_eq!(
            classify_failure_code("Database already open. Cannot acquire lock."),
            ServerErrorCode::StorageDbLocked
        );
    }

    #[test]
    fn classifies_missing_merged_doc_as_storage_not_found() {
        assert_eq!(
            classify_failure_code("Document not found while merging"),
            ServerErrorCode::StorageNotFound
        );
    }

    #[test]
    fn classifies_repo_scope_drift_as_repo_context_invalid() {
        assert_eq!(
            classify_failure_code("Repository UUID not resolved for selector wiki"),
            ServerErrorCode::ScRepoContextInvalid
        );
    }

    #[test]
    fn classifies_remote_bootstrap_drift_as_repo_context_invalid() {
        assert_eq!(
            classify_failure_code("Cannot bootstrap local repo while on remote branch"),
            ServerErrorCode::ScRepoContextInvalid
        );
    }
}
