//! plan_ref:
//!   - 04_repository#repo-scope-runtime
//!
use super::super::contexts::BranchContext;
use super::super::types::CoreState;

pub(super) fn build_branch_context(state: &CoreState) -> BranchContext {
    let scope = &state.runtime_clients.scope;
    BranchContext {
        active_branch: scope.active_branch,
        set_active_branch: scope.set_active_branch,
        on_switch_branch: scope.on_switch_branch,
        current_repo: scope.current_repo,
        set_current_repo: scope.set_current_repo,
        current_repo_id: scope.current_repo_id,
        set_current_repo_id: scope.set_current_repo_id,
        on_switch_repo: scope.on_switch_repo,
        on_create_repo: scope.on_create_repo,
        on_rename_repo: scope.on_rename_repo,
        on_remove_repo: scope.on_remove_repo,
        removal_preview: scope.removal_preview,
        on_confirm_remove_repo: scope.on_confirm_remove_repo,
        on_cancel_remove_repo: scope.on_cancel_remove_repo,
        shadow_repos: scope.shadow_repos,
        on_list_shadows: scope.on_list_shadows,
        repo_list: scope.repo_list,
        repo_entries: scope.repo_entries,
    }
}
