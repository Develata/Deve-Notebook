use crate::server::error_classify::{
    is_db_locked, is_repo_context_invalid, is_repo_not_selected, is_storage_corruption,
    is_storage_not_found,
};
use anyhow::Error;
use deve_core::protocol::{ServerError, ServerErrorCode};

pub const STALE_REMOTE_SCOPE_PREFIX: &str = "stale remote scope:";

pub fn stale_remote_scope_detail(detail: impl AsRef<str>) -> String {
    format!("{STALE_REMOTE_SCOPE_PREFIX} {}", detail.as_ref())
}

pub fn map_repo_scope_error(error: Error) -> ServerError {
    let detail = error.to_string();
    let lower = detail.to_ascii_lowercase();
    if is_repo_not_selected(&lower) {
        return ServerError::with_detail(ServerErrorCode::SyncRepoUnbound, detail);
    }
    if is_storage_not_found(&lower) {
        return ServerError::with_detail(ServerErrorCode::StorageNotFound, detail);
    }
    if is_db_locked(&lower) {
        return ServerError::with_detail(ServerErrorCode::StorageDbLocked, detail);
    }
    if is_storage_corruption(&lower) {
        return ServerError::with_detail(ServerErrorCode::StoragePersistFailed, detail);
    }
    if is_repo_context_invalid(&lower) {
        return ServerError::with_detail(ServerErrorCode::ScRepoContextInvalid, detail);
    }
    ServerError::with_detail(ServerErrorCode::RequestFailed, detail)
}

#[cfg(test)]
mod tests {
    use super::map_repo_scope_error;
    use deve_core::protocol::ServerErrorCode;

    #[test]
    fn classifies_ambiguous_remote_selector_as_repo_context_invalid() {
        let err = map_repo_scope_error(anyhow::anyhow!(
            "Ambiguous remote repository selector: shadow-wiki"
        ));
        assert_eq!(err.code, ServerErrorCode::ScRepoContextInvalid);
    }

    #[test]
    fn classifies_remote_workspace_access_as_repo_context_invalid() {
        let err = map_repo_scope_error(anyhow::anyhow!(
            "Local workspace path requested on remote branch: wiki"
        ));
        assert_eq!(err.code, ServerErrorCode::ScRepoContextInvalid);
    }

    #[test]
    fn classifies_missing_local_repo_name_as_storage_not_found() {
        let err = map_repo_scope_error(anyhow::anyhow!("Local repo not found for name wiki"));
        assert_eq!(err.code, ServerErrorCode::StorageNotFound);
    }

    #[test]
    fn classifies_broken_shadow_listing_as_storage_persist_failed() {
        let err = map_repo_scope_error(anyhow::anyhow!(
            "Broken shadow repo notes for peer peer-a while listing repos"
        ));
        assert_eq!(err.code, ServerErrorCode::StoragePersistFailed);
    }

    #[test]
    fn classifies_broken_repo_entry_as_storage_persist_failed() {
        let err = map_repo_scope_error(anyhow::anyhow!(
            "Broken repo entry \"/tmp/local/.redb\" while listing repos: invalid file stem"
        ));
        assert_eq!(err.code, ServerErrorCode::StoragePersistFailed);
    }

    #[test]
    fn classifies_missing_remote_catalog_as_storage_persist_failed() {
        let err = map_repo_scope_error(anyhow::anyhow!(
            "Broken remote repo catalog: remote repo directory missing at /tmp/ledger/remotes"
        ));
        assert_eq!(err.code, ServerErrorCode::StoragePersistFailed);
    }

    #[test]
    fn classifies_broken_remote_repo_as_storage_persist_failed() {
        let err = map_repo_scope_error(anyhow::anyhow!(
            "Broken remote repo wiki while resolving current repo URL before branch switch: repository URL missing"
        ));
        assert_eq!(err.code, ServerErrorCode::StoragePersistFailed);
    }

    #[test]
    fn classifies_missing_local_selector_as_repo_context_invalid() {
        let err = map_repo_scope_error(anyhow::anyhow!(
            "Local repository selector not resolved for stale-name"
        ));
        assert_eq!(err.code, ServerErrorCode::ScRepoContextInvalid);
    }

    #[test]
    fn classifies_repo_metadata_decode_failure_as_storage_persist_failed() {
        let err = map_repo_scope_error(anyhow::anyhow!("failed to deserialize repo metadata"));
        assert_eq!(err.code, ServerErrorCode::StoragePersistFailed);
    }

    #[test]
    fn classifies_local_catalog_stat_failure_as_storage_persist_failed() {
        let err = map_repo_scope_error(anyhow::anyhow!(
            "Failed to stat local repo directory while resolving local selector: \"/tmp/ledger/local\""
        ));
        assert_eq!(err.code, ServerErrorCode::StoragePersistFailed);
    }

    #[test]
    fn classifies_unmapped_error_as_request_failed() {
        let err = map_repo_scope_error(anyhow::anyhow!("something completely unexpected"));
        assert_eq!(err.code, ServerErrorCode::RequestFailed);
    }

    #[test]
    fn classifies_url_coverage_validation_as_storage_persist_failed() {
        let err = map_repo_scope_error(anyhow::anyhow!(
            "Broken remote repo wiki while validating URL coverage: repository URL not resolved"
        ));
        assert_eq!(err.code, ServerErrorCode::StoragePersistFailed);
    }
}
