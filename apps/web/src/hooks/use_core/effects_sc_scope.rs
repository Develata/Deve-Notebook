//! plan_ref:
//!   - 05_diff_logic#source-control-runtime
//!   - 04_repository#repo-scope-runtime
//!
use crate::hooks::use_core::{PendingBranchSwitch, PendingBranchTarget, PendingRepoSwitch};
use leptos::prelude::*;

pub(crate) fn matches_current_repo(
    repo_id: &Option<uuid::Uuid>,
    current_repo_id: ReadSignal<Option<String>>,
    pending_repo_switch: Option<PendingRepoSwitch>,
) -> bool {
    if pending_repo_switch.is_some() {
        return false;
    }
    match (repo_id, current_repo_id.get_untracked()) {
        (Some(repo_id), Some(current_repo_id)) => current_repo_id == repo_id.to_string(),
        (Some(_), None) => false,
        (None, None) => true,
        (None, Some(_)) => false,
    }
}

pub(crate) fn matches_current_scope(
    repo_id: &Option<uuid::Uuid>,
    branch: &Option<deve_core::models::PeerId>,
    current_repo_id: ReadSignal<Option<String>>,
    active_branch: ReadSignal<Option<deve_core::models::PeerId>>,
    pending_branch_switch: ReadSignal<Option<PendingBranchSwitch>>,
    pending_repo_switch: ReadSignal<Option<PendingRepoSwitch>>,
) -> bool {
    if pending_branch_switch.get_untracked().is_some() {
        return false;
    }
    matches_current_repo(
        repo_id,
        current_repo_id,
        pending_repo_switch.get_untracked(),
    ) && match pending_branch_switch.get_untracked() {
        Some(pending) if pending.target() == &PendingBranchTarget::Local => branch.is_none(),
        Some(pending) => {
            let PendingBranchTarget::Shadow(peer_id) = pending.target() else {
                return false;
            };
            branch.as_ref().map(|peer| peer.as_str()) == Some(peer_id.as_str())
        }
        None => active_branch.get_untracked() == *branch,
    }
}

#[cfg(test)]
mod tests {
    use super::matches_current_scope;
    use crate::hooks::use_core::{PendingBranchSwitch, PendingBranchTarget, PendingRepoSwitch};
    use deve_core::models::PeerId;
    use leptos::prelude::*;

    #[test]
    fn rejects_sc_scope_messages_while_branch_switch_pending() {
        let runtime = leptos::reactive::owner::Owner::new();
        runtime.set();
        let repo_id = uuid::Uuid::new_v4();
        let (current_repo_id, _) = signal(Some(repo_id.to_string()));
        let (active_branch, _) = signal(None);
        let (pending_branch_switch, _) = signal(Some(PendingBranchSwitch::new(
            PendingBranchTarget::Local,
            1,
        )));
        let (pending_repo_switch, _) = signal(None::<PendingRepoSwitch>);

        assert!(!matches_current_scope(
            &Some(repo_id),
            &None,
            current_repo_id,
            active_branch,
            pending_branch_switch,
            pending_repo_switch,
        ));
    }

    #[test]
    fn accepts_sc_scope_messages_only_after_branch_switch_settles() {
        let runtime = leptos::reactive::owner::Owner::new();
        runtime.set();
        let repo_id = uuid::Uuid::new_v4();
        let (current_repo_id, _) = signal(Some(repo_id.to_string()));
        let (active_branch, _) = signal(Some(PeerId::new("peer-a")));
        let (pending_branch_switch, _) = signal(None::<PendingBranchSwitch>);
        let (pending_repo_switch, _) = signal(None::<PendingRepoSwitch>);

        assert!(matches_current_scope(
            &Some(repo_id),
            &Some(PeerId::new("peer-a")),
            current_repo_id,
            active_branch,
            pending_branch_switch,
            pending_repo_switch,
        ));
    }
}
