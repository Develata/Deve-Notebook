//! plan_ref:
//!   - 06_repository#repo-scope-runtime
//!
use crate::hooks::use_core::PendingBranchTarget;
use deve_core::models::PeerId;
use leptos::prelude::{GetUntracked, ReadSignal};

#[derive(Clone, Copy)]
pub struct LocalScopeSignals {
    pub current_repo_id: ReadSignal<Option<String>>,
    pub current_scope_nonce: ReadSignal<u64>,
    pub active_branch: ReadSignal<Option<PeerId>>,
    pub pending_branch_switch: ReadSignal<Option<PendingBranchTarget>>,
    pub pending_repo_switch: ReadSignal<Option<String>>,
}

pub fn stable_local_scope_nonce(signals: LocalScopeSignals) -> Option<u64> {
    stable_local_scope_ready(signals).then(|| signals.current_scope_nonce.get_untracked())
}

pub fn stable_peer_branch_scope_nonce(signals: LocalScopeSignals, peer_id: &PeerId) -> Option<u64> {
    stable_peer_branch_scope_ready(signals, peer_id)
        .then(|| signals.current_scope_nonce.get_untracked())
}

fn stable_local_scope_ready(signals: LocalScopeSignals) -> bool {
    signals.current_repo_id.get_untracked().is_some()
        && signals.active_branch.get_untracked().is_none()
        && signals.pending_branch_switch.get_untracked().is_none()
        && signals.pending_repo_switch.get_untracked().is_none()
}

fn stable_peer_branch_scope_ready(signals: LocalScopeSignals, peer_id: &PeerId) -> bool {
    signals.current_repo_id.get_untracked().is_some()
        && signals.active_branch.get_untracked().as_ref() == Some(peer_id)
        && signals.pending_branch_switch.get_untracked().is_none()
        && signals.pending_repo_switch.get_untracked().is_none()
}

#[cfg(test)]
mod tests {
    use super::{LocalScopeSignals, stable_local_scope_nonce, stable_peer_branch_scope_nonce};
    use crate::hooks::use_core::PendingBranchTarget;
    use deve_core::models::PeerId;
    use leptos::prelude::*;

    #[test]
    fn returns_scope_nonce_for_stable_local_scope() {
        let runtime = leptos::reactive::owner::Owner::new();
        runtime.set();
        let (current_repo_id, _) = signal(Some("repo-1".to_string()));
        let (current_scope_nonce, _) = signal(7u64);
        let (active_branch, _) = signal(None);
        let (pending_branch_switch, _) = signal(None::<PendingBranchTarget>);
        let (pending_repo_switch, _) = signal(None::<String>);

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
            (Some(PendingBranchTarget::Local), None),
            (None, Some("repo-2".to_string())),
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
    fn returns_scope_nonce_only_for_stable_local_scope() {
        let runtime = leptos::reactive::owner::Owner::new();
        runtime.set();
        let (current_repo_id, _) = signal(Some("repo-1".to_string()));
        let (current_scope_nonce, _) = signal(9u64);
        let (active_branch, _) = signal(None::<PeerId>);
        let (pending_branch_switch, _) = signal(None::<PendingBranchTarget>);
        let (pending_repo_switch, _) = signal(None::<String>);

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

        let (pending_repo_switch, _) = signal(Some("repo-2".to_string()));
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
    fn returns_scope_nonce_for_matching_stable_peer_branch_scope() {
        let runtime = leptos::reactive::owner::Owner::new();
        runtime.set();
        let (current_repo_id, _) = signal(Some("repo-1".to_string()));
        let (current_scope_nonce, _) = signal(11u64);
        let (active_branch, _) = signal(Some(PeerId::new("peer-a")));
        let (pending_branch_switch, _) = signal(None::<PendingBranchTarget>);
        let (pending_repo_switch, _) = signal(None::<String>);

        assert_eq!(
            stable_peer_branch_scope_nonce(
                LocalScopeSignals {
                    current_repo_id,
                    current_scope_nonce,
                    active_branch,
                    pending_branch_switch,
                    pending_repo_switch,
                },
                &PeerId::new("peer-a"),
            ),
            Some(11)
        );
    }

    #[test]
    fn rejects_peer_branch_scope_for_mismatched_or_switching_scope() {
        let runtime = leptos::reactive::owner::Owner::new();
        runtime.set();
        let (current_repo_id, _) = signal(Some("repo-1".to_string()));
        let (current_scope_nonce, _) = signal(11u64);
        let (active_branch, _) = signal(Some(PeerId::new("peer-a")));
        let (pending_branch_switch, _) =
            signal(Some(PendingBranchTarget::Shadow("peer-b".to_string())));
        let (pending_repo_switch, _) = signal(None::<String>);

        assert_eq!(
            stable_peer_branch_scope_nonce(
                LocalScopeSignals {
                    current_repo_id,
                    current_scope_nonce,
                    active_branch,
                    pending_branch_switch,
                    pending_repo_switch,
                },
                &PeerId::new("peer-a"),
            ),
            None
        );

        let (pending_branch_switch, _) = signal(None::<PendingBranchTarget>);
        assert_eq!(
            stable_peer_branch_scope_nonce(
                LocalScopeSignals {
                    current_repo_id,
                    current_scope_nonce,
                    active_branch,
                    pending_branch_switch,
                    pending_repo_switch,
                },
                &PeerId::new("peer-b"),
            ),
            None
        );
    }
}
