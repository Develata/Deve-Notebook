//! plan_ref:
//!   - 06_repository#repo-scope-runtime
//!
//! Stale repo scope cleanup helpers.

use super::repo_scope_error::STALE_REMOTE_SCOPE_PREFIX;
use deve_core::protocol::{ServerError, ServerErrorCode};

pub(crate) fn should_clear_stale_remote_scope(error: &ServerError) -> bool {
    match error.code {
        ServerErrorCode::SyncRepoUnbound => true,
        ServerErrorCode::ScStaleScope => true,
        ServerErrorCode::ScRepoContextInvalid => error
            .detail
            .as_deref()
            .is_some_and(|detail| detail.starts_with(STALE_REMOTE_SCOPE_PREFIX)),
        _ => false,
    }
}

pub(super) fn should_clear_stale_local_scope(error: &ServerError) -> bool {
    matches!(
        error.code,
        ServerErrorCode::ScRepoContextInvalid
            | ServerErrorCode::ScStaleScope
            | ServerErrorCode::StorageNotFound
    )
}

#[cfg(test)]
mod tests {
    use super::{should_clear_stale_local_scope, should_clear_stale_remote_scope};
    use deve_core::protocol::{ServerError, ServerErrorCode};

    #[test]
    fn clears_unbound_remote_scope() {
        assert!(should_clear_stale_remote_scope(&ServerError::new(
            ServerErrorCode::SyncRepoUnbound
        )));
    }

    #[test]
    fn clears_stale_remote_selector_context_errors() {
        assert!(should_clear_stale_remote_scope(&ServerError::with_detail(
            ServerErrorCode::ScRepoContextInvalid,
            "stale remote scope: Remote repository selector not resolved for wiki",
        )));
    }

    #[test]
    fn clears_exact_selector_repo_mismatch_errors() {
        assert!(should_clear_stale_remote_scope(&ServerError::with_detail(
            ServerErrorCode::ScRepoContextInvalid,
            "stale remote scope: Session repo mismatch: expected a, resolved b for exact repository selector wiki-2",
        )));
    }

    #[test]
    fn clears_prefixed_stale_remote_scope_errors() {
        assert!(should_clear_stale_remote_scope(&ServerError::with_detail(
            ServerErrorCode::ScStaleScope,
            "stale remote scope: selector drifted after branch switch",
        )));
    }

    #[test]
    fn preserves_non_stale_remote_context_errors() {
        assert!(!should_clear_stale_remote_scope(&ServerError::with_detail(
            ServerErrorCode::ScRepoContextInvalid,
            "Local workspace path requested on remote branch: wiki",
        )));
    }

    #[test]
    fn preserves_unprefixed_remote_context_errors() {
        assert!(!should_clear_stale_remote_scope(&ServerError::with_detail(
            ServerErrorCode::ScRepoContextInvalid,
            "Session repo mismatch: expected a, resolved b for exact repository selector wiki-2",
        )));
    }

    #[test]
    fn clears_unrecoverable_local_scope_errors() {
        assert!(should_clear_stale_local_scope(&ServerError::with_detail(
            ServerErrorCode::ScRepoContextInvalid,
            "Local repository selector not resolved for stale-name",
        )));
        assert!(should_clear_stale_local_scope(&ServerError::with_detail(
            ServerErrorCode::StorageNotFound,
            "Repository not found: notes",
        )));
        assert!(!should_clear_stale_local_scope(&ServerError::with_detail(
            ServerErrorCode::StoragePersistFailed,
            "failed to deserialize repo metadata",
        )));
    }
}
