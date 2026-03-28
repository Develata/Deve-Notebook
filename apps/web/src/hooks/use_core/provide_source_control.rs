use super::super::contexts::SourceControlContext;
use super::super::types::CoreState;

pub(super) fn build_source_control_context(state: &CoreState) -> SourceControlContext {
    SourceControlContext {
        staged_changes: state.staged_changes,
        unstaged_changes: state.unstaged_changes,
        commit_history: state.commit_history,
        current_repo_id: state.current_repo_id,
        active_branch: state.active_branch,
        pending_branch_switch: state.pending_branch_switch,
        pending_repo_switch: state.pending_repo_switch,
        on_get_changes: state.on_get_changes,
        on_stage_file: state.on_stage_file,
        on_stage_files: state.on_stage_files,
        on_unstage_file: state.on_unstage_file,
        on_unstage_files: state.on_unstage_files,
        on_discard_file: state.on_discard_file,
        on_commit: state.on_commit,
        on_get_history: state.on_get_history,
        diff_content: state.diff_content,
        set_diff_content: state.set_diff_content,
        on_get_doc_diff: state.on_get_doc_diff,
        commit_diff_result: state.commit_diff_result,
        on_resolve_conflict: state.on_resolve_conflict,
        on_get_commit_diff: state.on_get_commit_diff,
        on_commit_and_push: state.on_commit_and_push,
    }
}
