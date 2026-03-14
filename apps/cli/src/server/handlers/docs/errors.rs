use crate::server::channel::DualChannel;
use deve_core::protocol::{ServerError, ServerErrorCode};

fn send(ch: &DualChannel, code: ServerErrorCode, detail: impl Into<String>) {
    ch.send_protocol_error(ServerError::with_detail(code, detail));
}

pub(super) fn request_failed(ch: &DualChannel, detail: impl Into<String>) {
    send(ch, ServerErrorCode::RequestFailed, detail);
}

pub(super) fn remote_branch_readonly(ch: &DualChannel) {
    ch.send_protocol_error(ServerError::new(ServerErrorCode::ScRemoteBranchReadonly));
}

pub(super) fn storage_not_found(ch: &DualChannel, detail: impl Into<String>) {
    send(ch, ServerErrorCode::StorageNotFound, detail);
}

pub(super) fn storage_conflict(ch: &DualChannel, detail: impl Into<String>) {
    send(ch, ServerErrorCode::StorageConflict, detail);
}

pub(super) fn storage_persist_failed(ch: &DualChannel, detail: impl Into<String>) {
    send(ch, ServerErrorCode::StoragePersistFailed, detail);
}

pub(super) fn projection_refresh_failed(ch: &DualChannel, detail: impl Into<String>) {
    send(ch, ServerErrorCode::StoragePersistFailed, detail);
}

pub(super) fn classified_failure(ch: &DualChannel, detail: impl Into<String>) {
    let detail = detail.into();
    send(ch, classify_failure_code(&detail), detail);
}

fn classify_failure_code(detail: &str) -> ServerErrorCode {
    let lower = detail.to_ascii_lowercase();
    if lower.contains("node meta missing")
        || lower.contains("canonical node path resolution failed")
    {
        return ServerErrorCode::StoragePersistFailed;
    }
    if lower.contains("local workspace path requested on remote branch")
        || lower.contains("local workspace root requested on remote branch")
        || lower.contains("local repo operation requested on remote branch")
    {
        return ServerErrorCode::ScRepoContextInvalid;
    }
    if lower.contains("repository not found:") {
        return ServerErrorCode::StorageNotFound;
    }
    if lower.contains("database already open")
        || lower.contains("cannot acquire lock")
        || lower.contains("db locked")
        || lower.contains("database is locked")
    {
        return ServerErrorCode::StorageDbLocked;
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
}
