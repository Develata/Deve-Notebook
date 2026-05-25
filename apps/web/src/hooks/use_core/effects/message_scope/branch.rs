//! plan_ref:
//!   - 07_network#web-ws-runtime
//!   - 04_repository#repo-scope-runtime
//!
use crate::hooks::use_core::PendingBranchTarget;
use deve_core::models::PeerId;

pub(super) fn expected_branch_string(
    active_branch: Option<PeerId>,
    pending_branch_switch: Option<PendingBranchTarget>,
) -> Option<String> {
    expected_peer_branch(active_branch, pending_branch_switch).map(|peer_id| peer_id.to_string())
}

pub(super) fn expected_peer_branch(
    active_branch: Option<PeerId>,
    pending_branch_switch: Option<PendingBranchTarget>,
) -> Option<PeerId> {
    pending_branch_switch
        .map(|pending| match pending {
            PendingBranchTarget::Local => None,
            PendingBranchTarget::Shadow(peer_id) => Some(PeerId::new(peer_id)),
        })
        .unwrap_or(active_branch)
}
