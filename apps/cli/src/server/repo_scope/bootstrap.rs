//! plan_ref:
//!   - 04_repository#repo-scope-runtime
//!
//! Local repo bootstrap fallback for scoped sessions.

use crate::server::AppState;
use crate::server::repo_scope::RepoScopeFailure;
use crate::server::session::WsSession;
use anyhow::Result;
use deve_core::protocol::RepoReadiness;
use std::sync::Arc;

pub(super) fn fallback_local_repo_name(
    state: &Arc<AppState>,
    _session: &WsSession,
) -> Result<String> {
    let watcher_runtime = state.watcher_runtime_view();
    let mut mounted = state
        .repo
        .list_cataloged_local_repo_summaries()?
        .into_iter()
        .filter(|repo| watcher_runtime.repo_readiness(repo.repo_id) == RepoReadiness::Mounted);
    let Some(repo) = mounted.next() else {
        return Err(RepoScopeFailure::repo_unbound(
            "No mounted local repository selected for current session",
        )
        .into());
    };
    if mounted.next().is_some() {
        return Err(RepoScopeFailure::repo_unbound(
            "Active repository not selected for current session",
        )
        .into());
    }
    Ok(repo.execution_name)
}
