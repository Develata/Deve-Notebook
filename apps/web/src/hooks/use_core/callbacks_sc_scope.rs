//! plan_ref:
//!   - 07_diff_logic#source-control-runtime
//!   - 06_repository#repo-scope-runtime
//!
use super::callbacks_sc::SourceControlScopeSignals;
use leptos::prelude::GetUntracked;

pub(super) fn source_control_scope_nonce(scope: SourceControlScopeSignals) -> Option<u64> {
    if scope.current_repo_id.get_untracked().is_some()
        && scope.active_branch.get_untracked().is_none()
        && scope.pending_branch_switch.get_untracked().is_none()
        && scope.pending_repo_switch.get_untracked().is_none()
    {
        Some(scope.current_scope_nonce.get_untracked())
    } else {
        None
    }
}

pub(super) fn source_control_read_scope_nonce(scope: SourceControlScopeSignals) -> Option<u64> {
    if scope.current_repo_id.get_untracked().is_some()
        && scope.pending_branch_switch.get_untracked().is_none()
        && scope.pending_repo_switch.get_untracked().is_none()
    {
        Some(scope.current_scope_nonce.get_untracked())
    } else {
        None
    }
}

#[cfg(test)]
mod tests;
