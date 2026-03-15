use super::callbacks_sc::SourceControlScopeSignals;
use leptos::prelude::GetUntracked;

pub(super) fn source_control_scope_nonce(scope: SourceControlScopeSignals) -> Option<u64> {
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
mod tests {
    use super::source_control_scope_nonce;
    use crate::hooks::use_core::{PendingBranchTarget, callbacks_sc::SourceControlScopeSignals};
    use leptos::prelude::*;

    #[test]
    fn source_control_scope_requires_bound_repo_and_no_pending_switch() {
        let runtime = leptos::reactive::owner::Owner::new();
        runtime.set();
        let (current_repo_id, _) = signal(None::<String>);
        let (current_scope_nonce, _) = signal(7u64);
        let (pending_branch_switch, _) = signal(None::<PendingBranchTarget>);
        let (pending_repo_switch, _) = signal(None::<String>);
        assert_eq!(
            source_control_scope_nonce(SourceControlScopeSignals {
                current_repo_id,
                current_scope_nonce,
                pending_branch_switch,
                pending_repo_switch,
            }),
            None
        );
        let (current_repo_id, _) = signal(Some("repo-a".to_string()));
        let (pending_branch_switch, _) = signal(Some(PendingBranchTarget::Local));
        assert_eq!(
            source_control_scope_nonce(SourceControlScopeSignals {
                current_repo_id,
                current_scope_nonce,
                pending_branch_switch,
                pending_repo_switch,
            }),
            None
        );
        let (pending_branch_switch, _) = signal(None::<PendingBranchTarget>);
        let (pending_repo_switch, _) = signal(Some("repo-b".to_string()));
        assert_eq!(
            source_control_scope_nonce(SourceControlScopeSignals {
                current_repo_id,
                current_scope_nonce,
                pending_branch_switch,
                pending_repo_switch,
            }),
            None
        );
        let (pending_repo_switch, _) = signal(None::<String>);
        assert_eq!(
            source_control_scope_nonce(SourceControlScopeSignals {
                current_repo_id,
                current_scope_nonce,
                pending_branch_switch,
                pending_repo_switch,
            }),
            Some(7)
        );
    }
}
