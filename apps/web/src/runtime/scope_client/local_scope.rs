//! plan_ref:
//!   - 04_repository#repo-scope-runtime
//!   - 09_web_thin_client_ledger#write-readiness
//!
//! Local scope readiness helpers for Web runtime consumers.

use crate::runtime::domain::{PendingBranchSwitch, PendingRepoSwitch};
use deve_core::models::PeerId;
use leptos::prelude::{GetUntracked, ReadSignal};

#[derive(Clone, Copy)]
pub struct LocalScopeSignals {
    pub current_repo_id: ReadSignal<Option<String>>,
    pub current_scope_nonce: ReadSignal<u64>,
    pub active_branch: ReadSignal<Option<PeerId>>,
    pub pending_branch_switch: ReadSignal<Option<PendingBranchSwitch>>,
    pub pending_repo_switch: ReadSignal<Option<PendingRepoSwitch>>,
}

pub fn stable_local_scope_nonce(signals: LocalScopeSignals) -> Option<u64> {
    stable_local_scope_ready(signals).then(|| signals.current_scope_nonce.get_untracked())
}

fn stable_local_scope_ready(signals: LocalScopeSignals) -> bool {
    signals.current_repo_id.get_untracked().is_some()
        && signals.active_branch.get_untracked().is_none()
        && signals.pending_branch_switch.get_untracked().is_none()
        && signals.pending_repo_switch.get_untracked().is_none()
}

#[cfg(test)]
mod tests {
    use super::{LocalScopeSignals, stable_local_scope_nonce};
    use crate::runtime::domain::{PendingBranchSwitch, PendingBranchTarget, PendingRepoSwitch};
    use deve_core::models::PeerId;
    use leptos::prelude::*;

    #[test]
    fn returns_scope_nonce_for_stable_local_scope() {
        let runtime = leptos::reactive::owner::Owner::new();
        runtime.set();
        let (current_repo_id, _) = signal(Some("repo-1".to_string()));
        let (current_scope_nonce, _) = signal(7u64);
        let (active_branch, _) = signal(None);
        let (pending_branch_switch, _) = signal(None::<PendingBranchSwitch>);
        let (pending_repo_switch, _) = signal(None::<PendingRepoSwitch>);

        assert_eq!(
            stable_local_scope_nonce(LocalScopeSignals {
                current_repo_id,
                current_scope_nonce,
                active_branch,
                pending_branch_switch,
                pending_repo_switch,
            }),
            Some(7)
        );
    }

    #[test]
    fn returns_none_when_branch_or_repo_switch_is_pending() {
        let runtime = leptos::reactive::owner::Owner::new();
        runtime.set();
        let (current_repo_id, _) = signal(Some("repo-1".to_string()));
        let (current_scope_nonce, _) = signal(7u64);
        let (active_branch, _) = signal(None);

        for (pending_branch_switch, pending_repo_switch) in [
            (
                Some(PendingBranchSwitch::new(PendingBranchTarget::Local, 1)),
                None,
            ),
            (None, Some(PendingRepoSwitch::switch("repo-2", 1))),
        ] {
            let (pending_branch_switch, _) = signal(pending_branch_switch.clone());
            let (pending_repo_switch, _) = signal(pending_repo_switch.clone());

            assert_eq!(
                stable_local_scope_nonce(LocalScopeSignals {
                    current_repo_id,
                    current_scope_nonce,
                    active_branch,
                    pending_branch_switch,
                    pending_repo_switch,
                }),
                None
            );
        }
    }

    #[test]
    fn returns_none_without_repo_or_for_active_branch() {
        let runtime = leptos::reactive::owner::Owner::new();
        runtime.set();
        let (current_scope_nonce, _) = signal(7u64);
        let (pending_branch_switch, _) = signal(None::<PendingBranchSwitch>);
        let (pending_repo_switch, _) = signal(None::<PendingRepoSwitch>);

        let (missing_repo_id, _) = signal(None::<String>);
        let (active_branch, _) = signal(None::<PeerId>);
        assert_eq!(
            stable_local_scope_nonce(LocalScopeSignals {
                current_repo_id: missing_repo_id,
                current_scope_nonce,
                active_branch,
                pending_branch_switch,
                pending_repo_switch,
            }),
            None
        );

        let (current_repo_id, _) = signal(Some("repo-1".to_string()));
        let (active_branch, _) = signal(Some(PeerId::new("peer-a")));
        assert_eq!(
            stable_local_scope_nonce(LocalScopeSignals {
                current_repo_id,
                current_scope_nonce,
                active_branch,
                pending_branch_switch,
                pending_repo_switch,
            }),
            None
        );
    }

    #[test]
    fn returns_scope_nonce_only_for_stable_local_scope() {
        let runtime = leptos::reactive::owner::Owner::new();
        runtime.set();
        let (current_repo_id, _) = signal(Some("repo-1".to_string()));
        let (current_scope_nonce, _) = signal(9u64);
        let (active_branch, _) = signal(None::<PeerId>);
        let (pending_branch_switch, _) = signal(None::<PendingBranchSwitch>);
        let (pending_repo_switch, _) = signal(None::<PendingRepoSwitch>);

        assert_eq!(
            stable_local_scope_nonce(LocalScopeSignals {
                current_repo_id,
                current_scope_nonce,
                active_branch,
                pending_branch_switch,
                pending_repo_switch,
            }),
            Some(9)
        );

        let (pending_repo_switch, _) = signal(Some(PendingRepoSwitch::switch("repo-2", 1)));
        assert_eq!(
            stable_local_scope_nonce(LocalScopeSignals {
                current_repo_id,
                current_scope_nonce,
                active_branch,
                pending_branch_switch,
                pending_repo_switch,
            }),
            None
        );
    }
}
