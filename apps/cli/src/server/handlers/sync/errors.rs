use crate::server::channel::DualChannel;
use crate::server::error_classify::{
    is_db_locked, is_repo_context_invalid, is_repo_not_selected, is_storage_corruption,
    is_storage_not_found,
};
use deve_core::protocol::{ServerError, ServerErrorCode};

fn send(
    ch: &DualChannel,
    code: ServerErrorCode,
    detail: impl Into<String>,
    scope_nonce: Option<u64>,
) {
    ch.send_protocol_error_with_scope_nonce(ServerError::with_detail(code, detail), scope_nonce);
}

pub(super) fn request_failed(
    ch: &DualChannel,
    detail: impl Into<String>,
    scope_nonce: Option<u64>,
) {
    send(ch, ServerErrorCode::RequestFailed, detail, scope_nonce);
}

pub(super) fn classified_failure(
    ch: &DualChannel,
    detail: impl Into<String>,
    scope_nonce: Option<u64>,
) {
    let detail = detail.into();
    send(ch, classify_failure_code(&detail), detail, scope_nonce);
}

pub(super) fn storage_persist_failed(
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

pub(super) fn sync_apply_failed(
    ch: &DualChannel,
    detail: impl Into<String>,
    scope_nonce: Option<u64>,
) {
    let detail = detail.into();
    send(ch, classify_failure_code(&detail), detail, scope_nonce);
}

fn classify_failure_code(detail: &str) -> ServerErrorCode {
    let lower = detail.to_ascii_lowercase();
    if is_repo_not_selected(&lower) {
        return ServerErrorCode::SyncRepoUnbound;
    }
    if lower.contains("decrypt") || lower.contains("aead") {
        return ServerErrorCode::SyncDecryptFailed;
    }
    if is_storage_not_found(&lower) {
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
    if lower.contains("signature")
        || lower.contains("verify")
        || lower.contains("public key")
        || lower.contains("unauthenticated")
    {
        return ServerErrorCode::SyncPeerUnauthenticated;
    }
    if lower.contains("snapshot")
        || lower.contains("sync payload")
        || lower.contains("sync response")
        || lower.contains("sync engine")
    {
        return ServerErrorCode::StoragePersistFailed;
    }
    ServerErrorCode::RequestFailed
}

#[cfg(test)]
mod tests {
    use super::classify_failure_code;
    use deve_core::protocol::ServerErrorCode;

    #[test]
    fn classifies_signature_failures_as_peer_unauthenticated() {
        assert_eq!(
            classify_failure_code("Handshake failed: signature verification failed"),
            ServerErrorCode::SyncPeerUnauthenticated
        );
    }

    #[test]
    fn classifies_snapshot_generation_as_storage_persist_failed() {
        assert_eq!(
            classify_failure_code("Failed to generate snapshot for repo x"),
            ServerErrorCode::StoragePersistFailed
        );
    }

    #[test]
    fn classifies_missing_sync_scope_as_repo_unbound() {
        assert_eq!(
            classify_failure_code("Active repository not selected: multiple local repos exist"),
            ServerErrorCode::SyncRepoUnbound
        );
    }

    #[test]
    fn classifies_sync_repo_scope_drift_as_repo_context_invalid() {
        assert_eq!(
            classify_failure_code(
                "Repo selector mismatch: repo_id resolved to default, repo_name resolved to test"
            ),
            ServerErrorCode::ScRepoContextInvalid
        );
    }

    #[test]
    fn classifies_missing_sync_repo_as_storage_not_found() {
        assert_eq!(
            classify_failure_code("Repository not found: default"),
            ServerErrorCode::StorageNotFound
        );
    }

    #[test]
    fn classifies_ambiguous_local_selector_as_repo_context_invalid() {
        assert_eq!(
            classify_failure_code("Ambiguous local repository selector: wiki"),
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
    fn classifies_legacy_projection_breakage_as_storage_persist_failed() {
        assert_eq!(
            classify_failure_code(
                "Tracked document projection missing for legacy-mapped path: notes/legacy.md"
            ),
            ServerErrorCode::StoragePersistFailed
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
