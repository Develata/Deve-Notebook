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

#[allow(clippy::too_many_arguments)]
pub fn accepts_write_ready(
    repo_id: &str,
    branch: &Option<PeerId>,
    scope_nonce: u64,
    current_repo_id: Option<String>,
    active_branch: Option<PeerId>,
    pending_branch_switch: Option<PendingBranchTarget>,
    pending_repo_switch: Option<String>,
    handshake_scope_nonce: Option<u64>,
) -> bool {
    pending_repo_switch.is_none()
        && pending_branch_switch.is_none()
        && handshake_scope_nonce == Some(scope_nonce)
        && peer_branch_matches_scope(branch, active_branch.clone(), pending_branch_switch)
        && active_branch.is_none()
        && current_repo_id.as_deref() == Some(repo_id)
}

pub fn accepts_write_ready_message(
    repo_id: &str,
    branch: &Option<PeerId>,
    scope_nonce: u64,
    signals: CoreSignals,
) -> bool {
    accepts_write_ready(
        repo_id,
        branch,
        scope_nonce,
        signals.current_repo_id.get_untracked(),
        signals.active_branch.get_untracked(),
        signals.pending_branch_switch.get_untracked(),
        signals.pending_repo_switch.get_untracked(),
        signals.handshake_scope_nonce.get_untracked(),
    )
}

pub fn accepts_edit_rejected_message(scope_nonce: Option<u64>, signals: CoreSignals) -> bool {
    signals.pending_repo_switch.get_untracked().is_none()
        && signals.pending_branch_switch.get_untracked().is_none()
        && scope_nonce == Some(signals.current_scope_nonce.get_untracked())
}

pub fn accepts_protocol_error_message(
    scope_nonce: Option<u64>,
    switch_nonce: Option<u64>,
    signals: CoreSignals,
) -> bool {
    if scope_nonce.is_none() {
        let Some(switch_nonce) = switch_nonce else {
            return false;
        };
        return signals.pending_branch_switch_nonce.get_untracked() == Some(switch_nonce)
            || signals.pending_repo_switch_nonce.get_untracked() == Some(switch_nonce);
    }
    signals.pending_repo_switch.get_untracked().is_none()
        && signals.pending_branch_switch.get_untracked().is_none()
        && scope_nonce == Some(signals.current_scope_nonce.get_untracked())
}

#[cfg(test)]
#[path = "message_repo_scope_test.rs"]
mod tests;
