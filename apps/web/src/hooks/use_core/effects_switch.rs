use deve_core::models::PeerId;
use leptos::prelude::*;

use super::types::{PendingBranchTarget, RepoSwitchSignals};

#[cfg(test)]
#[path = "effects_switch_branch_test.rs"]
mod branch_tests;
#[cfg(test)]
#[path = "effects_switch_repo_test.rs"]
mod repo_tests;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RepoSwitchOutcome {
    pub accepted: bool,
    pub should_refresh: bool,
}

#[derive(Clone, Copy)]
pub struct BranchSwitchSignals {
    pub pending_branch_switch: ReadSignal<Option<PendingBranchTarget>>,
    pub pending_branch_switch_nonce: ReadSignal<Option<u64>>,
    pub set_pending_branch_switch: WriteSignal<Option<PendingBranchTarget>>,
    pub set_pending_branch_switch_nonce: WriteSignal<Option<u64>>,
    pub set_active_branch: WriteSignal<Option<PeerId>>,
}

pub fn handle_branch_switched(
    peer_id: Option<String>,
    success: bool,
    switch_nonce: Option<u64>,
    signals: BranchSwitchSignals,
) -> bool {
    let Some(pending) = signals.pending_branch_switch.get_untracked() else {
        leptos::logging::warn!("忽略无 pending 的 BranchSwitched: {:?}", peer_id);
        return false;
    };
    let next_target = peer_id
        .clone()
        .map(PendingBranchTarget::Shadow)
        .unwrap_or(PendingBranchTarget::Local);
    if pending != next_target || signals.pending_branch_switch_nonce.get_untracked() != switch_nonce
    {
        leptos::logging::warn!("忽略过期 BranchSwitched: {:?}", peer_id);
        return false;
    }
    signals.set_pending_branch_switch.set(None);
    signals.set_pending_branch_switch_nonce.set(None);
    if !success {
        leptos::logging::warn!("分支切换失败");
        return false;
    }

    let next_branch = peer_id.map(PeerId::new);
    signals.set_active_branch.set(next_branch);
    true
}

pub fn handle_repo_switched(
    name: String,
    uuid: String,
    switch_nonce: Option<u64>,
    signals: RepoSwitchSignals,
) -> RepoSwitchOutcome {
    let current_repo = signals.current_repo.get_untracked();
    let current_repo_id = signals.current_repo_id.get_untracked();
    let pending_nonce = signals.pending_repo_switch_nonce.get_untracked();
    let current_scope_nonce = signals.current_scope_nonce.get_untracked();
    match signals.pending_repo_switch.get_untracked() {
        Some(pending) if pending == name => {
            if pending_nonce != switch_nonce {
                leptos::logging::warn!("忽略过期 RepoSwitched: {}", name);
                return ignored_repo_switch();
            }
            signals.set_pending_repo_switch.set(None);
            signals.set_pending_repo_switch_nonce.set(None);
            if let Some(switch_nonce) = switch_nonce {
                signals.set_current_scope_nonce.set(switch_nonce);
            }
        }
        Some(_) => {
            leptos::logging::warn!("忽略过期 RepoSwitched: {}", name);
            return ignored_repo_switch();
        }
        None => {
            let rebinding_after_branch_switch = current_repo.is_none()
                && current_repo_id.is_none()
                && switch_nonce == Some(current_scope_nonce);
            let same_repo = current_repo.as_deref() == Some(name.as_str())
                && current_repo_id.as_deref() == Some(uuid.as_str());
            let newer_same_repo_scope =
                same_repo && switch_nonce.is_some_and(|nonce| nonce >= current_scope_nonce);
            if !rebinding_after_branch_switch
                && !newer_same_repo_scope
                && (pending_nonce != switch_nonce || !same_repo)
            {
                leptos::logging::warn!("忽略无 pending 的 RepoSwitched: {}", name);
                return ignored_repo_switch();
            }
            signals.set_pending_repo_switch_nonce.set(None);
            if let Some(switch_nonce) = switch_nonce {
                signals.set_current_scope_nonce.set(switch_nonce);
            }
        }
    }

    let same_repo = !uuid.is_empty() && current_repo_id.as_deref() == Some(uuid.as_str());
    let scope_changed = switch_nonce.is_some() && switch_nonce != Some(current_scope_nonce);
    signals.set_current_repo.set(Some(name));
    signals
        .set_current_repo_id
        .set((!uuid.is_empty()).then_some(uuid));
    if !same_repo {
        signals.set_current_doc.set(None);
    }
    RepoSwitchOutcome {
        accepted: true,
        should_refresh: scope_changed || !same_repo,
    }
}

fn ignored_repo_switch() -> RepoSwitchOutcome {
    RepoSwitchOutcome {
        accepted: false,
        should_refresh: false,
    }
}
