use crate::hooks::use_core::PendingBranchTarget;
use deve_core::models::PeerId;

pub fn should_refresh_shadow_list(
    pending_branch_switch: Option<PendingBranchTarget>,
    pending_repo_switch: Option<String>,
    has_inflight_shadow_list: bool,
) -> bool {
    pending_branch_switch.is_none() && pending_repo_switch.is_none() && !has_inflight_shadow_list
}

pub fn should_recover_local_branch_from_deleted_peer(
    peer_id: &PeerId,
    active_branch: Option<PeerId>,
    pending_branch_switch: Option<PendingBranchTarget>,
    pending_repo_switch: Option<String>,
) -> bool {
    pending_branch_switch.is_none()
        && pending_repo_switch.is_none()
        && active_branch.as_ref() == Some(peer_id)
}

// Invariant: 只有 authoritative ShadowList 缺失当前 shadow 分支时，前端才允许恢复本地分支。
pub fn should_recover_local_branch_from_shadow_list(
    shadows: &[String],
    active_branch: Option<PeerId>,
    pending_branch_switch: Option<PendingBranchTarget>,
    pending_repo_switch: Option<String>,
    authoritative_refresh: bool,
) -> bool {
    authoritative_refresh
        && pending_branch_switch.is_none()
        && pending_repo_switch.is_none()
        && active_branch
            .as_ref()
            .map(|peer| !shadows.iter().any(|entry| entry == peer.as_str()))
            .unwrap_or(false)
}
