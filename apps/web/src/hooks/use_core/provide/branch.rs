use super::super::contexts::BranchContext;
use super::super::types::CoreState;

pub(super) fn build_branch_context(state: &CoreState) -> BranchContext {
    BranchContext {
        active_branch: state.active_branch,
        set_active_branch: state.set_active_branch,
        on_switch_branch: state.on_switch_branch,
        current_repo: state.current_repo,
        set_current_repo: state.set_current_repo,
        current_repo_id: state.current_repo_id,
        set_current_repo_id: state.set_current_repo_id,
        on_switch_repo: state.on_switch_repo,
        shadow_repos: state.shadow_repos,
        on_list_shadows: state.on_list_shadows,
        repo_list: state.repo_list,
    }
}
