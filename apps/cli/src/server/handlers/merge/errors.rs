use crate::server::channel::DualChannel;
use crate::server::error_classify::{
    is_db_locked, is_repo_context_invalid, is_storage_corruption, is_storage_not_found,
};
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
    if is_storage_not_found(&lower)
        || lower.contains("no local repository matched the active remote repository")
    {
        return ServerErrorCode::StorageNotFound;
    }
    if is_db_locked(&lower) {
        return ServerErrorCode::StorageDbLocked;
    }
    if is_storage_corruption(&lower) {
        return ServerErrorCode::StoragePersistFailed;
    }
    if is_repo_context_invalid(&lower) {
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

    #[test]
    fn classifies_stale_scope_nonce_as_repo_context_invalid() {
        assert_eq!(
            classify_failure_code(
                "Browser SyncHello stale scope nonce: current_scope_nonce=9, requested_scope_nonce=7"
            ),
            ServerErrorCode::ScRepoContextInvalid
        );
    }

    #[test]
    fn classifies_broken_repo_entry_as_storage_persist_failed() {
        assert_eq!(
            classify_failure_code(
                "Broken repo entry \"/tmp/local/.redb\" while listing repos: invalid file stem"
            ),
            ServerErrorCode::StoragePersistFailed
        );
    }

    #[test]
    fn classifies_missing_remote_catalog_as_storage_persist_failed() {
        assert_eq!(
            classify_failure_code(
                "Broken remote repo catalog: remote repo directory missing at /tmp/ledger/remotes"
            ),
            ServerErrorCode::StoragePersistFailed
        );
    }
}
