//! plan_ref:
//!   - 05_network#web-ws-runtime
//!   - 06_repository#repo-scope-runtime
//!
use deve_core::models::PeerId;
use leptos::prelude::{GetUntracked, Set};

use super::{BranchSwitchSignals, PendingBranchTarget};

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
