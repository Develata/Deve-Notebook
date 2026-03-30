use crate::hooks::use_core::PendingBranchTarget;
use crate::hooks::use_core::state::CoreSignals;
use deve_core::models::PeerId;
use leptos::prelude::GetUntracked;

use super::super::message_scope::peer_branch_matches_scope;
use super::logic;

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
