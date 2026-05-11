//! plan_ref:
//!   - 05_network#web-ws-runtime
//!   - 06_repository#repo-scope-runtime
//!
use crate::hooks::use_core::PendingBranchTarget;
use crate::hooks::use_core::write_gate::{RepoWriteGateState, repo_source_control_read_block};
use deve_core::models::PeerId;

#[cfg(test)]
mod tests;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RefreshScope {
    repo_id: Option<String>,
    branch: Option<PeerId>,
    scope_nonce: u64,
}

pub fn capture_refresh_scope(
    repo_id: Option<String>,
    branch: Option<PeerId>,
    pending_branch_switch: Option<PendingBranchTarget>,
    pending_repo_switch: Option<String>,
    scope_nonce: u64,
) -> Option<RefreshScope> {
    if branch.is_some() || pending_branch_switch.is_some() || pending_repo_switch.is_some() {
        return None;
    }
    Some(RefreshScope {
        repo_id,
        branch,
        scope_nonce,
    })
}

pub fn should_send_refresh(
    scope: &RefreshScope,
    repo_id: Option<String>,
    branch: Option<PeerId>,
    pending_branch_switch: Option<PendingBranchTarget>,
    pending_repo_switch: Option<String>,
    scope_nonce: u64,
) -> bool {
    pending_branch_switch.is_none()
        && pending_repo_switch.is_none()
        && scope.repo_id == repo_id
        && scope.branch == branch
        && scope.scope_nonce == scope_nonce
}

pub fn should_send_refresh_through_read_gate(
    scope: &RefreshScope,
    repo_id: Option<String>,
    branch: Option<PeerId>,
    pending_branch_switch: Option<PendingBranchTarget>,
    pending_repo_switch: Option<String>,
    scope_nonce: u64,
    gate_state: RepoWriteGateState<'_>,
) -> bool {
    should_send_refresh(
        scope,
        repo_id,
        branch,
        pending_branch_switch,
        pending_repo_switch,
        scope_nonce,
    ) && repo_source_control_read_block(gate_state).is_none()
}
