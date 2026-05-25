//! plan_ref:
//!   - 04_repository#repo-scope-runtime
//!
//! Branch switch current-repo hint construction.

use crate::server::AppState;
use crate::server::session::WsSession;
use deve_core::models::{PeerId, RepoId};
use std::sync::Arc;

use super::switcher_scope::CurrentBranchSwitchContext;

pub(super) struct BranchSwitchSelectorInput {
    pub(super) had_current_repo_hint: bool,
    pub(super) current_repo_id: Option<RepoId>,
    pub(super) current_repo_name: Option<String>,
    pub(super) current_repo_url: Option<String>,
}

pub(super) fn build_branch_switch_selector_input(
    state: &Arc<AppState>,
    session: &WsSession,
    raw_current_repo_hint: bool,
    current: &CurrentBranchSwitchContext,
    target_branch: Option<&PeerId>,
) -> BranchSwitchSelectorInput {
    if target_branch.is_none()
        && let Some(local_hint) = resolve_last_local_selector(state, session)
    {
        return BranchSwitchSelectorInput {
            had_current_repo_hint: true,
            current_repo_id: Some(local_hint.repo_id),
            current_repo_name: Some(local_hint.repo_name),
            current_repo_url: None,
        };
    }

    BranchSwitchSelectorInput {
        had_current_repo_hint: current.scope.is_some()
            || (raw_current_repo_hint && target_branch.is_some()),
        current_repo_id: current.scope.as_ref().map(|scope| scope.repo_id),
        current_repo_name: current.scope.as_ref().map(|scope| scope.repo_name.clone()),
        current_repo_url: current.repo_url.clone(),
    }
}

struct LastLocalSelector {
    repo_id: RepoId,
    repo_name: String,
}

fn resolve_last_local_selector(
    state: &Arc<AppState>,
    session: &WsSession,
) -> Option<LastLocalSelector> {
    session.active_branch.as_ref()?;

    let repo_name = state
        .repo
        .resolve_local_repo_name_for_execution(
            session.last_local_repo_id,
            session.last_local_repo.as_deref(),
        )
        .ok()?;
    let repo_id = state
        .repo
        .get_repo_info_for(None, Some(&repo_name))
        .ok()??
        .uuid;
    Some(LastLocalSelector { repo_id, repo_name })
}
