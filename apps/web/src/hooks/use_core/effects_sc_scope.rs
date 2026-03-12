use crate::hooks::use_core::PendingBranchTarget;
use leptos::prelude::*;

pub(crate) fn matches_current_repo(
    repo_id: &Option<uuid::Uuid>,
    current_repo_id: ReadSignal<Option<String>>,
    pending_repo_switch: Option<String>,
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
    pending_branch_switch: ReadSignal<Option<PendingBranchTarget>>,
    pending_repo_switch: ReadSignal<Option<String>>,
) -> bool {
    matches_current_repo(
        repo_id,
        current_repo_id,
        pending_repo_switch.get_untracked(),
    ) && match pending_branch_switch.get_untracked() {
        Some(PendingBranchTarget::Local) => branch.is_none(),
        Some(PendingBranchTarget::Shadow(peer_id)) => {
            branch.as_ref().map(|peer| peer.as_str()) == Some(peer_id.as_str())
        }
        None => active_branch.get_untracked() == *branch,
    }
}
