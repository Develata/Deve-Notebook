//! plan_ref:
//!   - 06_repository#repo-scope-runtime
//!
//! Local/remote counterpart lookup for repo-scoped sessions.

use super::ResolvedRepo;
use super::lookup::resolve_repo_by_name;
use crate::server::AppState;
use anyhow::{Result, anyhow};
use std::sync::Arc;

/// 将当前 resolved scope 收敛到本地可写仓库。
/// Invariants: 已处于本地分支时直接返回当前 scope；远端影子仓库只允许按 `RepoUUID -> URL` 收敛到本地仓库；无可写本地对应仓库时显式返回 `None`。
pub(super) fn resolve_local_counterpart_repo(
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
        return Err(anyhow!(
            "Broken remote repo {} while resolving local counterpart: repository URL missing",
            scope.repo_name
        ));
    };
    let Some(repo_name) = state.repo.find_local_repo_name_by_url(&url)? else {
        return Ok(None);
    };
    resolve_repo_by_name(state, None, None, repo_name).map(Some)
}
