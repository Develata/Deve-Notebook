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

pub fn resolve_session_repo(state: &Arc<AppState>, session: &WsSession) -> Result<ResolvedRepo> {
    let repo_name = session
        .active_repo
        .clone()
        .unwrap_or_else(|| state.repo.local_repo_name().to_string());
    let branch = session.active_branch.clone();
    let repo_id = session.active_repo_id.or_else(|| {
        state
            .repo
            .get_repo_info_for(branch.as_ref(), Some(&repo_name))
            .ok()
            .flatten()
            .map(|info| info.uuid)
    });
    let repo_id =
        repo_id.ok_or_else(|| anyhow!("Repository UUID not resolved for {}", repo_name))?;
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
