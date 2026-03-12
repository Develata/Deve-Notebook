//! 会话级 repo 解析辅助。
//!
//! Invariants:
//! - 进入底层 DB/Tree 算子前，必须先拿到真实 `RepoId`。
//! - 本地写路径不得静默回退到进程默认主库。

#[path = "repo_scope_lookup.rs"]
mod lookup;

use crate::server::AppState;
use crate::server::session::WsSession;
use anyhow::{Result, anyhow};
use deve_core::models::{PeerId, RepoId};
use deve_core::protocol::{ServerError, ServerErrorCode};
use redb::Database;
use std::sync::Arc;

use self::lookup::{resolve_repo_by_name, resolve_repo_by_repo_id};

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
    let branch = session.active_branch.clone();
    match resolve_repo_by_name(state, branch.clone(), session.active_repo_id, repo_name) {
        Ok(scope) => Ok(scope),
        Err(err) if err.to_string().starts_with("Session repo mismatch:") => {
            if branch.is_some()
                && let Some(repo_id) = session.active_repo_id
            {
                tracing::warn!("Recovering remote session repo scope from UUID: {}", err);
                return resolve_repo_by_repo_id(state, branch, repo_id);
            }
            if session.active_repo.is_some() {
                tracing::warn!("Recovering from stale local session repo_id: {}", err);
                let repo_name = session
                    .active_repo
                    .clone()
                    .expect("stale local repo recovery requires active_repo");
                return resolve_repo_by_name(state, None, None, repo_name);
            }
            Err(err)
        }
        Err(err) => Err(err),
    }
}

/// 解析并回写会话中的 repo 绑定，收敛 stale `active_repo_id/name`。
///
/// Invariants:
/// - 任何基于会话的 repo-scoped 读写，在落到底层算子前都应尽量先调用本函数。
/// - 若解析出的 `RepoId/RepoName` 与会话不一致，以解析结果为准回写会话。
pub fn resolve_session_repo_and_sync(
    state: &Arc<AppState>,
    session: &mut WsSession,
) -> Result<ResolvedRepo> {
    let scope = resolve_session_repo(state, session)?;
    if session.active_repo.as_deref() != Some(scope.repo_name.as_str())
        || session.active_repo_id != Some(scope.repo_id)
    {
        session.switch_repo(scope.repo_name.clone(), Some(scope.repo_id));
    }
    Ok(scope)
}

/// 将当前 resolved scope 收敛到本地可写仓库。
///
/// Invariants:
/// - 已处于本地分支时直接返回当前 scope。
/// - 远端影子仓库必须优先按 `RepoUUID` 匹配本地仓库。
/// - 若 UUID 不可用，才允许按共享 URL 回退解析。
/// - 无可写本地对应仓库时，调用方必须显式处理 `None`。
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

pub fn map_repo_scope_error(error: anyhow::Error) -> ServerError {
    let detail = error.to_string();
    let lower = detail.to_ascii_lowercase();
    if lower.contains("active repository not selected") {
        return ServerError::with_detail(ServerErrorCode::SyncRepoUnbound, detail);
    }
    if contains_any(
        &lower,
        &[
            "remote session lost repo name",
            "repository uuid not resolved",
            "session repo mismatch",
            "repo selector mismatch",
            "local repo not found for uuid",
        ],
    ) {
        return ServerError::with_detail(ServerErrorCode::ScRepoContextInvalid, detail);
    }
    ServerError::with_detail(ServerErrorCode::RequestFailed, detail)
}

fn resolve_repo_name_from_session(
    state: &Arc<AppState>,
    session: &WsSession,
) -> Result<Option<String>> {
    if session.active_branch.is_none() {
        if let Some(repo_name) = session.active_repo.clone() {
            if state
                .repo
                .get_repo_info_for(None, Some(&repo_name))
                .ok()
                .flatten()
                .is_some()
            {
                return Ok(Some(repo_name));
            }
            if let Some(repo_id) = session.active_repo_id
                && let Some(resolved) = state.repo.find_local_repo_name_by_id(repo_id)?
            {
                tracing::warn!(
                    "Recovering local repo name from UUID: repo_id={}, stale_name={:?}, resolved_name={}",
                    repo_id,
                    session.active_repo,
                    resolved
                );
                return Ok(Some(resolved));
            }
            return Ok(Some(repo_name));
        }
        if let Some(repo_id) = session.active_repo_id {
            return state.repo.find_local_repo_name_by_id(repo_id);
        }
        return Ok(None);
    }
    if let Some(repo_id) = session.active_repo_id
        && let Some(branch) = session.active_branch.as_ref()
    {
        let info = state
            .repo
            .get_repo_info_for(Some(branch), Some(&repo_id.to_string()))?;
        if let Some(info) = info {
            if session.active_repo.as_deref() != Some(info.name.as_str()) {
                tracing::warn!(
                    "Recovering remote repo name from UUID: branch={}, repo_id={}, stale_name={:?}, resolved_name={}",
                    branch,
                    repo_id,
                    session.active_repo,
                    info.name
                );
            }
            return Ok(Some(info.name));
        }
    }
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

fn contains_any(input: &str, patterns: &[&str]) -> bool {
    patterns.iter().any(|pattern| input.contains(pattern))
}
