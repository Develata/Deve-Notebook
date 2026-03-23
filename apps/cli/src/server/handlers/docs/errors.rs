use crate::server::channel::DualChannel;
use crate::server::repo_scope::map_repo_scope_error;
use anyhow::anyhow;
use deve_core::protocol::{ServerError, ServerErrorCode};

fn send(
    ch: &DualChannel,
    code: ServerErrorCode,
    detail: impl Into<String>,
    scope_nonce: Option<u64>,
) {
    ch.send_protocol_error_with_scope_nonce(ServerError::with_detail(code, detail), scope_nonce);
}

pub(super) fn request_failed_scoped(
    ch: &DualChannel,
    detail: impl Into<String>,
    scope_nonce: Option<u64>,
) {
    send(ch, ServerErrorCode::RequestFailed, detail, scope_nonce);
}

pub(super) fn remote_branch_readonly_scoped(ch: &DualChannel, scope_nonce: Option<u64>) {
    ch.send_protocol_error_with_scope_nonce(
        ServerError::new(ServerErrorCode::ScRemoteBranchReadonly),
        scope_nonce,
    );
}

pub(super) fn storage_not_found_scoped(
    ch: &DualChannel,
    detail: impl Into<String>,
    scope_nonce: Option<u64>,
) {
    send(ch, ServerErrorCode::StorageNotFound, detail, scope_nonce);
}

pub(super) fn storage_conflict_scoped(
    ch: &DualChannel,
    detail: impl Into<String>,
    scope_nonce: Option<u64>,
) {
    send(ch, ServerErrorCode::StorageConflict, detail, scope_nonce);
}

pub(super) fn storage_persist_failed_scoped(
    ch: &DualChannel,
    detail: impl Into<String>,
    scope_nonce: Option<u64>,
) {
    send(
        ch,
        ServerErrorCode::StoragePersistFailed,
        detail,
        scope_nonce,
    );
}

pub(super) fn projection_refresh_failed_scoped(
    ch: &DualChannel,
    detail: impl Into<String>,
    scope_nonce: Option<u64>,
) {
    send(
        ch,
        ServerErrorCode::StoragePersistFailed,
        detail,
        scope_nonce,
    );
}

pub(super) fn classified_failure_scoped(
    ch: &DualChannel,
    detail: impl Into<String>,
    scope_nonce: Option<u64>,
) {
    let detail = detail.into();
    send(ch, classify_failure_code(&detail), detail, scope_nonce);
}

fn classify_failure_code(detail: &str) -> ServerErrorCode {
    let repo_scope_code = map_repo_scope_error(anyhow!(detail.to_string())).code;
    if repo_scope_code != ServerErrorCode::RequestFailed {
        return repo_scope_code;
    }
    let lower = detail.to_ascii_lowercase();
    if lower.contains("source not found:") {
        return ServerErrorCode::StorageNotFound;
    }
    if lower.contains("node meta missing")
        || lower.contains("tracked document projection missing")
        || lower.contains("canonical node path resolution failed")
    {
        return ServerErrorCode::StoragePersistFailed;
    }
    if lower.contains("already exists")
        || lower.contains("destination exists")
        || lower.contains("target file already exists on disk")
    {
        return ServerErrorCode::StorageConflict;
    }
    ServerErrorCode::RequestFailed
}

#[cfg(test)]
mod tests {
    use super::classify_failure_code;
    use deve_core::protocol::ServerErrorCode;

    #[test]
    fn classifies_node_meta_breakage_as_storage_persist_failed() {
        assert_eq!(
            classify_failure_code("Node meta missing for node abc"),
            ServerErrorCode::StoragePersistFailed
        );
    }

    #[test]
    fn classifies_remote_workspace_access_as_repo_context_invalid() {
        assert_eq!(
            classify_failure_code("Local workspace path requested on remote branch: wiki"),
            ServerErrorCode::ScRepoContextInvalid
        );
    }

    #[test]
    fn classifies_missing_repo_selection_as_sync_repo_unbound() {
        assert_eq!(
            classify_failure_code("Active repository not selected: multiple local repos exist"),
            ServerErrorCode::SyncRepoUnbound
        );
    }

    #[test]
    fn classifies_missing_docs_as_storage_not_found() {
        assert_eq!(
            classify_failure_code("Document not found: abc"),
            ServerErrorCode::StorageNotFound
        );
        assert_eq!(
            classify_failure_code("Source not found: notes/a.md"),
            ServerErrorCode::StorageNotFound
        );
    }

    #[test]
    fn classifies_existing_targets_as_storage_conflict() {
        assert_eq!(
            classify_failure_code("Target file already exists on disk: notes/a.md"),
            ServerErrorCode::StorageConflict
        );
    }

    #[test]
    fn classifies_legacy_projection_breakage_as_storage_persist_failed() {
        assert_eq!(
            classify_failure_code(
                "Tracked document projection missing for legacy-mapped path: notes/legacy.md"
            ),
            ServerErrorCode::StoragePersistFailed
        );
    }
}
