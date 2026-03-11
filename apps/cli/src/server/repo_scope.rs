//! 会话级 repo 解析辅助。
//!
//! Invariants:
//! - 进入底层 DB/Tree 算子前，必须先拿到真实 `RepoId`。
//! - 本地写路径不得静默回退到进程默认主库。

use crate::server::AppState;
use crate::server::session::WsSession;
use anyhow::{Result, anyhow};
use deve_core::models::{PeerId, RepoId};
use redb::Database;
use std::sync::Arc;

#[derive(Clone)]
pub struct ResolvedRepo {
    pub repo_id: RepoId,
    pub repo_name: String,
    pub branch: Option<PeerId>,
}

/// 仅允许首次本地引导时回退到主本地库。
///
/// Invariants:
/// - 只在 `active_branch == None` 时允许默认回退。
/// - 引导完成后，后续路径应统一通过 `resolve_session_repo`。
pub fn bootstrap_local_repo(state: &Arc<AppState>, session: &WsSession) -> Result<ResolvedRepo> {
    if session.active_branch.is_some() {
        return Err(anyhow!(
            "Cannot bootstrap local repo while on remote branch"
        ));
    }
    let repo_name = resolve_repo_name_from_session(state, session)?
        .unwrap_or_else(|| state.repo.local_repo_name().to_string());
    resolve_repo_by_name(state, None, session.active_repo_id, repo_name)
}

pub fn resolve_session_repo(state: &Arc<AppState>, session: &WsSession) -> Result<ResolvedRepo> {
    let repo_name = resolve_repo_name_from_session(state, session)?
        .ok_or_else(|| anyhow!("Active repository not selected for current session"))?;
    match resolve_repo_by_name(
        state,
        session.active_branch.clone(),
        session.active_repo_id,
        repo_name,
    ) {
        Ok(scope) => Ok(scope),
        Err(err)
            if session.active_branch.is_none()
                && session.active_repo.is_some()
                && err.to_string().starts_with("Session repo mismatch:") =>
        {
            tracing::warn!("Recovering from stale local session repo_id: {}", err);
            resolve_repo_by_name(
                state,
                None,
                None,
                session.active_repo.clone().unwrap_or_default(),
            )
        }
        Err(err) => Err(err),
    }
}

fn resolve_repo_name_from_session(
    state: &Arc<AppState>,
    session: &WsSession,
) -> Result<Option<String>> {
    if let Some(repo_name) = session.active_repo.clone() {
        return Ok(Some(repo_name));
    }
    let Some(repo_id) = session.active_repo_id else {
        return Ok(None);
    };
    if session.active_branch.is_some() {
        return Err(anyhow!(
            "Remote session lost repo name for bound repo {}",
            repo_id
        ));
    }
    state.repo.find_local_repo_name_by_id(repo_id)
}

fn resolve_repo_by_name(
    state: &Arc<AppState>,
    branch: Option<PeerId>,
    expected_repo_id: Option<RepoId>,
    repo_name: String,
) -> Result<ResolvedRepo> {
    let branch_ref = branch.as_ref();
    let info = state
        .repo
        .get_repo_info_for(branch_ref, Some(&repo_name))?
        .ok_or_else(|| anyhow!("Repository UUID not resolved for {}", repo_name))?;
    let repo_id = info.uuid;
    if let Some(expected_repo_id) = expected_repo_id
        && expected_repo_id != repo_id
    {
        return Err(anyhow!(
            "Session repo mismatch: expected {}, resolved {} for {}",
            expected_repo_id,
            repo_id,
            repo_name
        ));
    }
    Ok(ResolvedRepo {
        repo_id,
        repo_name,
        branch,
    })
}

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
