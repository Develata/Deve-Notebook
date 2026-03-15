use crate::server::channel::DualChannel;
use deve_core::protocol::{ServerError, ServerErrorCode};

fn send(ch: &DualChannel, code: ServerErrorCode, detail: impl Into<String>) {
    ch.send_protocol_error(ServerError::with_detail(code, detail));
}

pub(super) fn engine_unavailable(ch: &DualChannel) {
    send(
        ch,
        ServerErrorCode::StoragePersistFailed,
        "Failed to get or create sync engine",
    );
}

pub(super) fn request_failed(ch: &DualChannel, detail: impl Into<String>) {
    send(ch, ServerErrorCode::RequestFailed, detail);
}

pub(super) fn classified_failure(ch: &DualChannel, detail: impl Into<String>) {
    let detail = detail.into();
    send(ch, classify_failure_code(&detail), detail);
}

pub(super) fn storage_persist_failed(ch: &DualChannel, detail: impl Into<String>) {
    send(ch, ServerErrorCode::StoragePersistFailed, detail);
}

pub(super) fn sync_apply_failed(ch: &DualChannel, detail: impl Into<String>) {
    let detail = detail.into();
    send(ch, classify_failure_code(&detail), detail);
}

fn classify_failure_code(detail: &str) -> ServerErrorCode {
    let lower = detail.to_ascii_lowercase();
    if contains_any(
        &lower,
        &[
            "active repository not selected",
            "multiple local repos exist",
            "no local repositories available",
        ],
    ) {
        return ServerErrorCode::SyncRepoUnbound;
    }
    if lower.contains("decrypt") || lower.contains("aead") {
        return ServerErrorCode::SyncDecryptFailed;
    }
    if contains_any(
        &lower,
        &[
            "repository not found:",
            "document not found",
            "doc not found",
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
            "scope mismatch",
            "stale scope nonce",
        ],
    ) {
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

fn contains_any(input: &str, patterns: &[&str]) -> bool {
    patterns.iter().any(|pattern| input.contains(pattern))
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
}
