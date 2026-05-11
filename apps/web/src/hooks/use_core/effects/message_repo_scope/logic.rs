//! plan_ref:
//!   - 05_network#web-ws-runtime
//!   - 06_repository#repo-scope-runtime
//!
use crate::hooks::use_core::PendingBranchTarget;

pub(super) fn switches_are_idle(
    pending_branch_switch: Option<&PendingBranchTarget>,
    pending_repo_switch: Option<&str>,
) -> bool {
    pending_branch_switch.is_none() && pending_repo_switch.is_none()
}

pub(super) fn current_repo_matches(
    repo_id: &Option<uuid::Uuid>,
    current_repo_id: Option<String>,
) -> bool {
    match (repo_id, current_repo_id) {
        (Some(repo_id), Some(current_repo_id)) => current_repo_id == repo_id.to_string(),
        (Some(_), None) => false,
        (None, None) => true,
        (None, Some(_)) => false,
    }
}

pub(super) fn accepts_current_scope_message(
    scope_nonce: Option<u64>,
    current_scope_nonce: u64,
    pending_branch_switch: Option<PendingBranchTarget>,
    pending_repo_switch: Option<String>,
) -> bool {
    switches_are_idle(
        pending_branch_switch.as_ref(),
        pending_repo_switch.as_deref(),
    ) && scope_nonce == Some(current_scope_nonce)
}

pub(super) fn accepts_switch_protocol_error(
    switch_nonce: Option<u64>,
    pending_branch_switch_nonce: Option<u64>,
    pending_repo_switch_nonce: Option<u64>,
) -> bool {
    let Some(switch_nonce) = switch_nonce else {
        return false;
    };
    pending_branch_switch_nonce == Some(switch_nonce)
        || pending_repo_switch_nonce == Some(switch_nonce)
}
