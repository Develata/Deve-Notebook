//! plan_ref:
//!   - 04_repository#repo-scope-runtime
//!
//! Workspace paths derived from resolved repo scope.

use super::{AppState, ResolvedRepo};
use anyhow::{Result, anyhow};
use deve_core::protocol::{ServerError, ServerErrorCode};
use redb::Database;
use std::sync::Arc;

pub fn run_on_resolved_local_repo<F, R>(
    state: &Arc<AppState>,
    repo: &ResolvedRepo,
    f: F,
) -> Result<R>
where
    F: FnOnce(&Database) -> Result<R>,
{
    if repo.branch.is_some() {
        return Err(anyhow!(
            "Local repo operation requested on remote branch: {}",
            repo.repo_name
        ));
    }
    state.repo.run_on_local_repo(&repo.repo_name, f)
}

pub fn local_repo_path(
    state: &Arc<AppState>,
    repo: &ResolvedRepo,
    rel_path: &str,
) -> Result<std::path::PathBuf> {
    if repo.branch.is_some() {
        return Err(anyhow!(
            "Local workspace path requested on remote branch: {}",
            repo.repo_name
        ));
    }
    state
        .repo
        .local_repo_workspace_path(&repo.repo_name, rel_path)
}

pub fn local_repo_root(state: &Arc<AppState>, repo: &ResolvedRepo) -> Result<std::path::PathBuf> {
    if repo.branch.is_some() {
        return Err(anyhow!(
            "Local workspace root requested on remote branch: {}",
            repo.repo_name
        ));
    }
    state.repo.local_repo_workspace_root(&repo.repo_name)
}

pub fn ensure_resolved_local_repo_writable(
    state: &Arc<AppState>,
    repo: &ResolvedRepo,
) -> std::result::Result<(), ServerError> {
    if repo.branch.is_some() {
        return Err(ServerError::with_detail(
            ServerErrorCode::ScRemoteBranchReadonly,
            repo.repo_name.clone(),
        ));
    }
    ensure_local_repo_projection_writable(state, &repo.repo_name)
}

pub fn ensure_local_repo_projection_writable(
    state: &Arc<AppState>,
    repo_name: &str,
) -> std::result::Result<(), ServerError> {
    if state.sync_manager.is_projection_degraded(repo_name) {
        return Err(degraded_local_repo_write_error(repo_name));
    }
    Ok(())
}

pub fn degraded_local_repo_write_error(repo_name: &str) -> ServerError {
    ServerError::with_detail(
        ServerErrorCode::StoragePersistFailed,
        format!("Local repo projection is degraded; repair before writing: {repo_name}"),
    )
}
