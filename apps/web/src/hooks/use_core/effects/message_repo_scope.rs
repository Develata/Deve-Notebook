use crate::hooks::use_core::PendingBranchTarget;
use crate::hooks::use_core::state::CoreSignals;
use deve_core::models::PeerId;
use leptos::prelude::*;

use super::super::effects_sc_scope::matches_current_repo;
use super::message_scope::peer_branch_matches_scope;
#[path = "message_repo_scope_logic.rs"]
mod logic;

pub fn matches_repo_scope(
    repo_id: &Option<uuid::Uuid>,
    branch: &Option<PeerId>,
    current_repo_id: ReadSignal<Option<String>>,
    active_branch: Option<PeerId>,
    pending_branch_switch: Option<PendingBranchTarget>,
    pending_repo_switch: Option<String>,
) -> bool {
    logic::switches_are_idle(
        pending_branch_switch.as_ref(),
        pending_repo_switch.as_deref(),
    ) && matches_current_repo(repo_id, current_repo_id, None)
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

pub fn matches_projection_message_scope(
    repo_id: &Option<uuid::Uuid>,
    branch: &Option<PeerId>,
    signals: CoreSignals,
) -> bool {
    peer_branch_matches_scope(
        branch,
        signals.active_branch.get_untracked(),
        signals.pending_branch_switch.get_untracked(),
    ) && logic::current_repo_matches(repo_id, signals.current_repo_id.get_untracked())
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
    logic::switches_are_idle(
        pending_branch_switch.as_ref(),
        pending_repo_switch.as_deref(),
    ) && handshake_scope_nonce == Some(scope_nonce)
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
    logic::accepts_current_scope_message(
        scope_nonce,
        signals.current_scope_nonce.get_untracked(),
        signals.pending_branch_switch.get_untracked(),
        signals.pending_repo_switch.get_untracked(),
    )
}

pub fn accepts_protocol_error_message(
    scope_nonce: Option<u64>,
    switch_nonce: Option<u64>,
    signals: CoreSignals,
) -> bool {
    if scope_nonce.is_none() {
        return logic::accepts_switch_protocol_error(
            switch_nonce,
            signals.pending_branch_switch_nonce.get_untracked(),
            signals.pending_repo_switch_nonce.get_untracked(),
        );
    }
    logic::accepts_current_scope_message(
        scope_nonce,
        signals.current_scope_nonce.get_untracked(),
        signals.pending_branch_switch.get_untracked(),
        signals.pending_repo_switch.get_untracked(),
    )
}

#[cfg(test)]
#[path = "message_repo_scope_test.rs"]
mod tests;
