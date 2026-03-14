use crate::hooks::use_core::PendingBranchTarget;
use deve_core::models::PeerId;
use leptos::prelude::{GetUntracked, ReadSignal};

#[derive(Clone, Copy)]
pub struct LocalScopeSignals {
    pub current_repo_id: ReadSignal<Option<String>>,
    pub active_branch: ReadSignal<Option<PeerId>>,
    pub pending_branch_switch: ReadSignal<Option<PendingBranchTarget>>,
    pub pending_repo_switch: ReadSignal<Option<String>>,
}

pub fn run_if_stable_local_scope(signals: LocalScopeSignals, op_name: &str, action: impl FnOnce()) {
    if !stable_local_scope_ready(signals) {
        leptos::logging::warn!("忽略 {}: local repo scope 尚未稳定", op_name);
        return;
    }
    action();
}

fn stable_local_scope_ready(signals: LocalScopeSignals) -> bool {
    signals.current_repo_id.get_untracked().is_some()
        && signals.active_branch.get_untracked().is_none()
        && signals.pending_branch_switch.get_untracked().is_none()
        && signals.pending_repo_switch.get_untracked().is_none()
}

#[cfg(test)]
mod tests {
    use super::{LocalScopeSignals, run_if_stable_local_scope};
    use crate::hooks::use_core::PendingBranchTarget;
    use leptos::prelude::*;

    #[test]
    fn runs_only_in_stable_local_scope() {
        let runtime = leptos::reactive::owner::Owner::new();
        runtime.set();
        let (current_repo_id, _) = signal(Some("repo-1".to_string()));
        let (active_branch, _) = signal(None);
        let (pending_branch_switch, _) = signal(None::<PendingBranchTarget>);
        let (pending_repo_switch, _) = signal(None::<String>);
        let (ran, set_ran) = signal(false);

        run_if_stable_local_scope(
            LocalScopeSignals {
                current_repo_id,
                active_branch,
                pending_branch_switch,
                pending_repo_switch,
            },
            "test-op",
            move || set_ran.set(true),
        );

        assert!(ran.get_untracked());
    }

    #[test]
    fn blocks_when_branch_or_repo_switch_is_pending() {
        let runtime = leptos::reactive::owner::Owner::new();
        runtime.set();
        let (current_repo_id, _) = signal(Some("repo-1".to_string()));
        let (active_branch, _) = signal(None);
        let (ran, set_ran) = signal(false);

        for (pending_branch_switch, pending_repo_switch) in [
            (Some(PendingBranchTarget::Local), None),
            (None, Some("repo-2".to_string())),
        ] {
            let (pending_branch_switch, _) = signal(pending_branch_switch.clone());
            let (pending_repo_switch, _) = signal(pending_repo_switch.clone());

            run_if_stable_local_scope(
                LocalScopeSignals {
                    current_repo_id,
                    active_branch,
                    pending_branch_switch,
                    pending_repo_switch,
                },
                "test-op",
                move || set_ran.set(true),
            );
        }

        assert!(!ran.get_untracked());
    }
}
