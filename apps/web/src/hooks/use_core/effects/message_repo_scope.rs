use crate::hooks::use_core::PendingBranchTarget;
use crate::hooks::use_core::state::CoreSignals;
use deve_core::models::PeerId;
use leptos::prelude::*;

use super::super::effects_sc_scope::matches_current_repo;
use super::message_scope::peer_branch_matches_scope;

pub fn matches_repo_scope(
    repo_id: &Option<uuid::Uuid>,
    branch: &Option<PeerId>,
    current_repo_id: ReadSignal<Option<String>>,
    active_branch: Option<PeerId>,
    pending_branch_switch: Option<PendingBranchTarget>,
    pending_repo_switch: Option<String>,
) -> bool {
    pending_repo_switch.is_none()
        && pending_branch_switch.is_none()
        && matches_current_repo(repo_id, current_repo_id, None)
        && peer_branch_matches_scope(branch, active_branch, pending_branch_switch)
}

pub fn matches_current_message_scope(
    repo_id: &Option<uuid::Uuid>,
    branch: &Option<PeerId>,
    signals: CoreSignals,
) -> bool {
    matches_repo_scope(
        repo_id,
        branch,
        signals.current_repo_id,
        signals.active_branch.get_untracked(),
        signals.pending_branch_switch.get_untracked(),
        signals.pending_repo_switch.get_untracked(),
    )
}

pub fn accepts_write_ready(
    repo_id: &str,
    branch: &Option<PeerId>,
    current_repo_id: Option<String>,
    active_branch: Option<PeerId>,
    pending_branch_switch: Option<PendingBranchTarget>,
    pending_repo_switch: Option<String>,
) -> bool {
    pending_repo_switch.is_none()
        && pending_branch_switch.is_none()
        && peer_branch_matches_scope(branch, active_branch.clone(), pending_branch_switch)
        && active_branch.is_none()
        && current_repo_id.as_deref() == Some(repo_id)
}

pub fn accepts_write_ready_message(
    repo_id: &str,
    branch: &Option<PeerId>,
    signals: CoreSignals,
) -> bool {
    accepts_write_ready(
        repo_id,
        branch,
        signals.current_repo_id.get_untracked(),
        signals.active_branch.get_untracked(),
        signals.pending_branch_switch.get_untracked(),
        signals.pending_repo_switch.get_untracked(),
    )
}

#[cfg(test)]
mod tests {
    use super::{accepts_write_ready, matches_repo_scope};
    use crate::hooks::use_core::PendingBranchTarget;
    use deve_core::models::PeerId;
    use leptos::prelude::*;

    #[test]
    fn rejects_repo_scoped_messages_while_repo_switch_pending() {
        let runtime = leptos::reactive::owner::Owner::new();
        runtime.set();
        let repo_id = uuid::Uuid::new_v4();
        let (current_repo_id, _) = signal(Some(repo_id.to_string()));
        assert!(!matches_repo_scope(
            &Some(repo_id),
            &None,
            current_repo_id,
            None,
            None,
            Some("test".into()),
        ));
    }

    #[test]
    fn rejects_repo_scoped_messages_while_branch_switch_pending() {
        let runtime = leptos::reactive::owner::Owner::new();
        runtime.set();
        let repo_id = uuid::Uuid::new_v4();
        let (current_repo_id, _) = signal(Some(repo_id.to_string()));
        assert!(!matches_repo_scope(
            &Some(repo_id),
            &Some(PeerId::new("peer-a")),
            current_repo_id,
            None,
            Some(PendingBranchTarget::Shadow("peer-a".into())),
            None,
        ));
    }

    #[test]
    fn rejects_write_ready_while_repo_switch_pending() {
        let repo_id = uuid::Uuid::new_v4().to_string();
        assert!(!accepts_write_ready(
            &repo_id,
            &None,
            Some(repo_id.clone()),
            None,
            Some(PendingBranchTarget::Local),
            Some("default".into()),
        ));
    }

    #[test]
    fn rejects_write_ready_while_branch_switch_pending() {
        let repo_id = uuid::Uuid::new_v4().to_string();
        assert!(!accepts_write_ready(
            &repo_id,
            &None,
            Some(repo_id.clone()),
            None,
            Some(PendingBranchTarget::Local),
            None,
        ));
    }

    #[test]
    fn accepts_write_ready_only_for_local_branch_and_bound_repo() {
        let repo_id = uuid::Uuid::new_v4().to_string();
        assert!(accepts_write_ready(
            &repo_id,
            &None,
            Some(repo_id.clone()),
            None,
            None,
            None,
        ));
        assert!(!accepts_write_ready(
            &repo_id,
            &Some(PeerId::new("peer-a")),
            Some(repo_id.clone()),
            Some(PeerId::new("peer-a")),
            None,
            None,
        ));
    }
}
