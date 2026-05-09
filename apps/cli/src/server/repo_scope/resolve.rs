//! plan_ref:
//!   - 06_repository#repo-scope-runtime
//!
//! Session repo scope resolution and fail-closed validation.

use super::lookup::resolve_repo_by_name;
use super::bootstrap::fallback_local_repo_name;
use super::error::RepoScopeFailure;
use super::selector::resolve_repo_name_from_session;
use super::stale_remote_scope_detail;
use crate::server::AppState;
use crate::server::session::WsSession;
use crate::server::shadow_scope;
use anyhow::Result;
use deve_core::models::{PeerId, RepoId};
use std::sync::Arc;

#[derive(Clone, Debug)]
pub struct ResolvedRepo {
    pub repo_id: RepoId,
    pub repo_name: String,
    pub branch: Option<PeerId>,
}

pub fn stale_unbound_remote_scope_detail(branch: &PeerId) -> String {
    stale_remote_scope_detail(format!(
        "Active repository not selected for remote branch {} while runtime binding was still present",
        branch
    ))
}

/// 仅允许首次本地引导时回退到主本地库。
/// Invariants: 只在 `active_branch == None` 时允许默认回退；引导完成后统一走 `resolve_session_repo`。
pub fn bootstrap_local_repo(state: &Arc<AppState>, session: &WsSession) -> Result<ResolvedRepo> {
    if session.active_branch.is_some() {
        return Err(RepoScopeFailure::repo_context_invalid(
            "Cannot bootstrap local repo while on remote branch",
        )
        .into());
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
    let repo_name = match resolve_repo_name_from_session(state, session)? {
        Some(repo_name) => repo_name,
        None => {
            if let Some(branch) = session.active_branch.as_ref() {
                shadow_scope::ensure_remote_branch_available(state, branch)?;
                if session.has_runtime_scope_binding() {
                    return Err(
                        RepoScopeFailure::stale_scope(stale_unbound_remote_scope_detail(branch))
                            .into(),
                    );
                }
            }
            return Err(RepoScopeFailure::repo_unbound(
                "Active repository not selected for current session",
            )
            .into());
        }
    };
    let branch = session.active_branch.clone();
    resolve_repo_by_name(state, branch, session.active_repo_id, repo_name)
}
