use deve_core::models::PeerId;
use leptos::prelude::*;

use super::types::{PendingBranchTarget, RepoSwitchSignals};

#[cfg(test)]
#[path = "effects_switch_test.rs"]
mod tests;

pub fn handle_branch_switched(
    peer_id: Option<String>,
    success: bool,
    switch_nonce: Option<u64>,
    active_branch: ReadSignal<Option<PeerId>>,
    pending_branch_switch: ReadSignal<Option<PendingBranchTarget>>,
    pending_branch_switch_nonce: ReadSignal<Option<u64>>,
    set_pending_branch_switch: WriteSignal<Option<PendingBranchTarget>>,
    set_pending_branch_switch_nonce: WriteSignal<Option<u64>>,
    set_active_branch: WriteSignal<Option<PeerId>>,
) -> bool {
    let Some(pending) = pending_branch_switch.get_untracked() else {
        leptos::logging::warn!("忽略无 pending 的 BranchSwitched: {:?}", peer_id);
        return false;
    };
    let next_target = peer_id
        .clone()
        .map(PendingBranchTarget::Shadow)
        .unwrap_or(PendingBranchTarget::Local);
    if pending != next_target || pending_branch_switch_nonce.get_untracked() != switch_nonce {
        leptos::logging::warn!("忽略过期 BranchSwitched: {:?}", peer_id);
        return false;
    }
    set_pending_branch_switch.set(None);
    set_pending_branch_switch_nonce.set(None);
    if !success {
        leptos::logging::warn!("分支切换失败");
        return false;
    }

    let next_branch = peer_id.map(PeerId::new);
    let changed = active_branch.get_untracked() != next_branch;
    set_active_branch.set(next_branch);
    changed
}

pub fn handle_repo_switched(
    name: String,
    uuid: String,
    switch_nonce: Option<u64>,
    signals: RepoSwitchSignals,
) -> bool {
    let current_repo = signals.current_repo.get_untracked();
    let current_repo_id = signals.current_repo_id.get_untracked();
    let pending_nonce = signals.pending_repo_switch_nonce.get_untracked();
    match signals.pending_repo_switch.get_untracked() {
        Some(pending) if pending == name => {
            if pending_nonce != switch_nonce {
                leptos::logging::warn!("忽略过期 RepoSwitched: {}", name);
                return false;
            }
            signals.set_pending_repo_switch.set(None);
            signals.set_pending_repo_switch_nonce.set(None);
        }
        Some(_) => {
            leptos::logging::warn!("忽略过期 RepoSwitched: {}", name);
            return false;
        }
        None => {
            let rebinding_after_branch_switch = current_repo.is_none() && current_repo_id.is_none();
            let same_repo = current_repo.as_deref() == Some(name.as_str())
                && current_repo_id.as_deref() == Some(uuid.as_str());
            if (!same_repo && !rebinding_after_branch_switch) || pending_nonce != switch_nonce {
                leptos::logging::warn!("忽略无 pending 的 RepoSwitched: {}", name);
                return false;
            }
            signals.set_pending_repo_switch_nonce.set(None);
        }
    }

    let same_repo = !uuid.is_empty() && current_repo_id.as_deref() == Some(uuid.as_str());
    signals.set_current_repo.set(Some(name));
    signals
        .set_current_repo_id
        .set((!uuid.is_empty()).then_some(uuid));
    if !same_repo {
        signals.set_current_doc.set(None);
    }
    !same_repo
}
