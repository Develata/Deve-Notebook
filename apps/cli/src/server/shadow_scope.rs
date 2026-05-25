//! plan_ref:
//!   - 07_network#server-ws-runtime
//!   - 04_repository#repo-scope-runtime

use crate::server::AppState;
use crate::server::repo_scope::RepoScopeFailure;
use crate::server::session::WsSession;
use deve_core::models::PeerId;
use deve_core::protocol::{ServerError, ServerErrorCode};
use std::path::PathBuf;
use std::sync::Arc;

pub(crate) fn ensure_remote_branch_available(
    state: &Arc<AppState>,
    branch: &PeerId,
) -> Result<(), RepoScopeFailure> {
    let peer_dir = checked_remotes_dir(state)?.join(branch.to_filename());
    match peer_dir.try_exists() {
        Ok(true) => {}
        Ok(false) => {
            return Err(RepoScopeFailure::remote_branch_unavailable(branch));
        }
        Err(err) => {
            return Err(RepoScopeFailure::storage_persist_failed(format!(
                "Failed to stat remote peer directory {:?} while validating branch availability: {}",
                peer_dir, err
            )));
        }
    }
    if !std::fs::metadata(&peer_dir)
        .map_err(|err| {
            RepoScopeFailure::storage_persist_failed(format!(
                "Failed to read remote peer directory metadata {:?} while validating branch availability: {}",
                peer_dir,
                err
            ))
        })?
        .is_dir()
    {
        return Err(RepoScopeFailure::storage_persist_failed(format!(
            "Broken shadow peer {} while validating branch availability: expected directory",
            branch
        )));
    }
    Ok(())
}

fn checked_remotes_dir(state: &Arc<AppState>) -> Result<PathBuf, RepoScopeFailure> {
    let remotes_dir = state.repo.remotes_dir();
    match remotes_dir.try_exists() {
        Ok(true) => {}
        Ok(false) => {
            return Err(RepoScopeFailure::storage_persist_failed(format!(
                "Broken remote repo catalog: remote repo directory missing at {:?}",
                remotes_dir
            )));
        }
        Err(err) => {
            return Err(RepoScopeFailure::storage_persist_failed(format!(
                "Broken remote repo catalog: failed to stat remote repo directory {:?}: {}",
                remotes_dir, err
            )));
        }
    }
    if !std::fs::metadata(&remotes_dir)
        .map_err(|err| {
            RepoScopeFailure::storage_persist_failed(format!(
                "Broken remote repo catalog: failed to read remote repo directory metadata {:?}: {}",
                remotes_dir,
                err
            ))
        })?
        .is_dir()
    {
        return Err(RepoScopeFailure::storage_persist_failed(format!(
            "Broken remote repo catalog: expected directory at {:?}",
            remotes_dir
        )));
    }
    Ok(remotes_dir)
}

pub(crate) fn map_remote_branch_availability(
    state: &Arc<AppState>,
    branch: &PeerId,
) -> Result<(), ServerError> {
    ensure_remote_branch_available(state, branch).map_err(ServerError::from)
}

pub(crate) fn should_clear_missing_remote_branch(error: &ServerError) -> bool {
    matches!(
        error.code,
        ServerErrorCode::ScRepoContextInvalid | ServerErrorCode::ScStaleScope
    ) && error
        .detail
        .as_deref()
        .is_some_and(|detail| detail.contains("Remote branch not available:"))
}

pub(crate) fn clear_stale_remote_branch(session: &mut WsSession) {
    session.switch_branch(None);
    session.clear_active_repo();
    session.clear_active_db();
    session.clear_sync_binding();
}
