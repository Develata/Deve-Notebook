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
    if lower.contains("repository not found:")
        || lower.contains("no local repository matched the active remote repository")
    {
        return ServerErrorCode::StorageNotFound;
    }
    if lower.contains("database already open")
        || lower.contains("cannot acquire lock")
        || lower.contains("db locked")
        || lower.contains("database is locked")
    {
        return ServerErrorCode::StorageDbLocked;
    }
    if lower.contains("local repo operation requested on remote branch")
        || lower.contains("repo selector mismatch")
        || lower.contains("remote session lost repo name")
    {
        return ServerErrorCode::ScRepoContextInvalid;
    }
    ServerErrorCode::RequestFailed
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
}
