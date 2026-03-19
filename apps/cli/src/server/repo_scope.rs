//! 会话级 repo 解析辅助。
//!
//! Invariants:
//! - 进入底层 DB/Tree 算子前，必须先拿到真实 `RepoId`。
//! - 本地写路径不得静默回退到进程默认主库。

#[path = "repo_scope_lookup.rs"]
mod lookup;
#[path = "repo_scope_bootstrap.rs"]
mod repo_scope_bootstrap;
#[path = "repo_scope_cleanup.rs"]
mod repo_scope_cleanup;
#[path = "repo_scope_error.rs"]
mod repo_scope_error;
#[path = "repo_scope_remote.rs"]
mod repo_scope_remote;
#[path = "repo_scope_selector.rs"]
mod repo_scope_selector;
#[path = "repo_scope_workspace.rs"]
mod repo_scope_workspace;

use crate::server::AppState;
use crate::server::session::WsSession;
use anyhow::{Result, anyhow};
use deve_core::models::{PeerId, RepoId};
use deve_core::protocol::ServerErrorCode;
use std::sync::Arc;

use self::lookup::resolve_repo_by_name;
use self::repo_scope_bootstrap::fallback_local_repo_name;
use self::repo_scope_cleanup::{should_clear_stale_local_scope, should_clear_stale_remote_scope};
pub use self::repo_scope_error::map_repo_scope_error;
use self::repo_scope_selector::resolve_repo_name_from_session;
pub use self::repo_scope_workspace::{
    local_repo_path, local_repo_root, run_on_resolved_local_repo,
};

#[derive(Clone, Debug)]
pub struct ResolvedRepo {
    pub repo_id: RepoId,
    pub repo_name: String,
    pub branch: Option<PeerId>,
}

/// 仅允许首次本地引导时回退到主本地库。
/// Invariants: 只在 `active_branch == None` 时允许默认回退；引导完成后统一走 `resolve_session_repo`。
pub fn bootstrap_local_repo(state: &Arc<AppState>, session: &WsSession) -> Result<ResolvedRepo> {
    if session.active_branch.is_some() {
        return Err(anyhow!(
            "Cannot bootstrap local repo while on remote branch"
        ));
    }
    if session.active_repo.is_some() || session.active_repo_id.is_some() {
        return resolve_session_repo(state, session);
    }
    let repo_name = match resolve_repo_name_from_session(state, session)? {
        Some(repo_name) => repo_name,
        None => fallback_local_repo_name(state, session)?,
    };
    resolve_repo_by_name(state, None, session.active_repo_id, repo_name)
}

pub fn resolve_session_repo(state: &Arc<AppState>, session: &WsSession) -> Result<ResolvedRepo> {
    let repo_name = resolve_repo_name_from_session(state, session)?
        .ok_or_else(|| anyhow!("Active repository not selected for current session"))?;
    let branch = session.active_branch.clone();
    resolve_repo_by_name(state, branch, session.active_repo_id, repo_name)
}

/// 解析并回写会话中的 repo 绑定，收敛 stale `active_repo_id/name`。
/// Invariants: 会话级 repo-scoped 读写应尽量先调用本函数；若解析结果与会话不一致，以解析结果为准。
pub fn resolve_session_repo_and_sync(
    state: &Arc<AppState>,
    session: &mut WsSession,
) -> Result<ResolvedRepo> {
    let scope = match resolve_session_repo(state, session) {
        Ok(scope) => scope,
        Err(err) => {
            let mapped = map_repo_scope_error(anyhow!(err.to_string()));
            let clear_stale_scope = if session.active_branch.is_some() {
                should_clear_stale_remote_scope(&mapped)
            } else {
                should_clear_stale_local_scope(&mapped)
                    || (mapped.code == ServerErrorCode::SyncRepoUnbound
                        && session.has_runtime_scope_binding())
            };
            if clear_stale_scope {
                session.clear_active_repo();
                session.clear_active_db();
                session.clear_sync_binding();
            }
            return Err(err);
        }
    };
    if session.active_repo.as_deref() != Some(scope.repo_name.as_str())
        || session.active_repo_id != Some(scope.repo_id)
    {
        session.switch_repo(scope.repo_name.clone(), Some(scope.repo_id));
    }
    Ok(scope)
}

/// 将当前 resolved scope 收敛到本地可写仓库。
/// Invariants: 已处于本地分支时直接返回当前 scope；远端影子仓库只允许按 `RepoUUID -> URL` 收敛到本地仓库；无可写本地对应仓库时显式返回 `None`。
pub fn resolve_local_counterpart_repo(
    state: &Arc<AppState>,
    scope: &ResolvedRepo,
) -> Result<Option<ResolvedRepo>> {
    if scope.branch.is_none() {
        return Ok(Some(scope.clone()));
    }
    if let Some(repo_name) = state.repo.find_local_repo_name_by_id(scope.repo_id)? {
        return resolve_repo_by_name(state, None, Some(scope.repo_id), repo_name).map(Some);
    }
    let Some(url) = state
        .repo
        .get_repo_url(scope.branch.as_ref(), &scope.repo_name)?
    else {
        return Ok(None);
    };
    let Some(repo_name) = state.repo.find_local_repo_name_by_url(&url)? else {
        return Ok(None);
    };
    resolve_repo_by_name(state, None, None, repo_name).map(Some)
}
