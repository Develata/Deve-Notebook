use crate::server::repo_scope::map_repo_scope_error;
use anyhow::anyhow;
use deve_core::models::PeerId;
use deve_core::protocol::{ServerError, ServerErrorCode};

pub(super) fn prepare_switch_error(branch: Option<&PeerId>, err: anyhow::Error) -> ServerError {
    let detail = format!("Failed to switch repo: {}", err);
    let mapped = map_repo_scope_error(anyhow!(detail.clone()));
    if mapped.code != ServerErrorCode::RequestFailed {
        return mapped;
    }
    let _ = branch;
    ServerError::with_detail(ServerErrorCode::StoragePersistFailed, detail)
}

#[cfg(test)]
mod tests {
    use super::prepare_switch_error;
    use deve_core::models::PeerId;
    use deve_core::protocol::ServerErrorCode;

    #[test]
    fn preserves_repo_scope_classification_for_remote_switch_prepare_failures() {
        let err = prepare_switch_error(
            Some(&PeerId::new("peer-a")),
            anyhow::anyhow!("Remote repository UUID not resolved for selector: wiki"),
        );
        assert_eq!(err.code, ServerErrorCode::ScRepoContextInvalid);
    }

    #[test]
    fn keeps_local_prepare_persistence_failures_as_storage_persist_failed() {
        let err = prepare_switch_error(None, anyhow::anyhow!("projection rebuild failed"));
        assert_eq!(err.code, ServerErrorCode::StoragePersistFailed);
    }

    #[test]
    fn keeps_remote_lock_failures_as_storage_db_locked() {
        let err = prepare_switch_error(
            Some(&PeerId::new("peer-a")),
            anyhow::anyhow!("Database already open. Cannot acquire lock."),
        );
        assert_eq!(err.code, ServerErrorCode::StorageDbLocked);
    }

    #[test]
    fn keeps_remote_non_lock_prepare_failures_as_storage_persist_failed() {
        let err = prepare_switch_error(
            Some(&PeerId::new("peer-a")),
            anyhow::anyhow!("permission denied while opening shadow repo"),
        );
        assert_eq!(err.code, ServerErrorCode::StoragePersistFailed);
    }
}
