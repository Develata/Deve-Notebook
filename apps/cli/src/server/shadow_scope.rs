use crate::server::AppState;
use crate::server::repo_scope::{map_repo_scope_error, stale_remote_scope_detail};
use crate::server::session::WsSession;
use anyhow::{Result, anyhow};
use deve_core::models::PeerId;
use deve_core::protocol::{ServerError, ServerErrorCode};
use std::path::PathBuf;
use std::sync::Arc;

pub(crate) fn ensure_remote_branch_available(state: &Arc<AppState>, branch: &PeerId) -> Result<()> {
    let peer_dir = checked_remotes_dir(state)?.join(branch.to_filename());
    match peer_dir.try_exists() {
        Ok(true) => {}
        Ok(false) => {
            return Err(anyhow!(
                "{}",
                stale_remote_scope_detail(format!("Remote branch not available: {}", branch))
            ));
        }
        Err(err) => {
            return Err(anyhow!(
                "Failed to stat remote peer directory {:?} while validating branch availability: {}",
                peer_dir,
                err
            ));
        }
    }
    if !std::fs::metadata(&peer_dir)
        .map_err(|err| {
            anyhow!(
                "Failed to read remote peer directory metadata {:?} while validating branch availability: {}",
                peer_dir,
                err
            )
        })?
        .is_dir()
    {
        return Err(anyhow!(
            "Broken shadow peer {} while validating branch availability: expected directory",
            branch
        ));
    }
    Ok(())
}

fn checked_remotes_dir(state: &Arc<AppState>) -> Result<PathBuf> {
    let remotes_dir = state.repo.remotes_dir();
    match remotes_dir.try_exists() {
        Ok(true) => {}
        Ok(false) => {
            return Err(anyhow!(
                "Broken remote repo catalog: remote repo directory missing at {:?}",
                remotes_dir
            ));
        }
        Err(err) => {
            return Err(anyhow!(
                "Broken remote repo catalog: failed to stat remote repo directory {:?}: {}",
                remotes_dir,
                err
            ));
        }
    }
    if !std::fs::metadata(&remotes_dir)
        .map_err(|err| {
            anyhow!(
                "Broken remote repo catalog: failed to read remote repo directory metadata {:?}: {}",
                remotes_dir,
                err
            )
        })?
        .is_dir()
    {
        return Err(anyhow!(
            "Broken remote repo catalog: expected directory at {:?}",
            remotes_dir
        ));
    }
    Ok(remotes_dir)
}

pub(crate) fn map_remote_branch_availability(
    state: &Arc<AppState>,
    branch: &PeerId,
) -> Result<(), ServerError> {
    ensure_remote_branch_available(state, branch).map_err(map_repo_scope_error)
}

pub(crate) fn should_clear_missing_remote_branch(error: &ServerError) -> bool {
    error.code == ServerErrorCode::ScRepoContextInvalid
        && error
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
