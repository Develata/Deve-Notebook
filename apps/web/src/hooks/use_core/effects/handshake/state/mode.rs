//! plan_ref:
//!   - 07_network#web-ws-runtime
//!   - 04_repository#repo-scope-runtime
//!
use crate::hooks::use_core::PendingBranchTarget;
use deve_core::models::PeerId;

pub(in super::super) fn handshake_mode_key(
    endpoint: &str,
    degraded: Option<()>,
    repo_id: Option<&str>,
    branch: Option<&PeerId>,
    scope_nonce: u64,
) -> Option<String> {
    degraded
        .map(|_| format!("{endpoint}::degraded::{scope_nonce}"))
        .or_else(|| {
            repo_id.map(|repo_id| {
                let branch_key = branch
                    .map(PeerId::to_string)
                    .unwrap_or_else(|| "local".to_string());
                format!("{endpoint}::{repo_id}::{branch_key}::{scope_nonce}")
            })
        })
}

pub(in super::super) fn suspended_handshake_mode_key(endpoint: &str) -> String {
    format!("{endpoint}::suspended")
}

pub(in super::super) fn restore_bootstrap_key(
    endpoint: &str,
    repo_name: Option<&str>,
    branch: Option<&PeerId>,
    _scope_nonce: u64,
    should_restore: bool,
    last_mode: Option<&str>,
) -> Option<String> {
    if !should_restore {
        return None;
    }
    let repo_key = repo_name.unwrap_or("unbound");
    let branch_key = branch
        .map(PeerId::to_string)
        .unwrap_or_else(|| "local".to_string());
    let restore_key = format!("{endpoint}::restore::{repo_key}::{branch_key}");
    (last_mode != Some(restore_key.as_str())).then_some(restore_key)
}

pub(in super::super) fn should_suspend_handshake(
    branch: &Option<PeerId>,
    pending_branch_switch: Option<&PendingBranchTarget>,
    pending_repo_switch: Option<&str>,
) -> bool {
    branch.is_some() || pending_branch_switch.is_some() || pending_repo_switch.is_some()
}

pub(in super::super) fn should_restore_session_scope(
    is_reconnect_bootstrap: bool,
    pending_branch_switch: Option<&PendingBranchTarget>,
    pending_repo_switch: Option<&str>,
) -> bool {
    is_reconnect_bootstrap && pending_branch_switch.is_none() && pending_repo_switch.is_none()
}
