//! plan_ref:
//!   - 07_network#web-ws-runtime
//!   - 04_repository#repo-scope-runtime
//!
use crate::hooks::use_core::PendingBranchTarget;
use crate::hooks::use_core::state::CoreSignals;
use deve_core::models::PeerId;
use leptos::prelude::GetUntracked;

use super::super::message_scope::peer_branch_matches_scope;
use super::logic;

pub struct WriteReadyScopeInput<'a> {
    pub repo_id: &'a str,
    pub branch: Option<PeerId>,
    pub scope_nonce: u64,
    pub current_repo_id: Option<String>,
    pub active_branch: Option<PeerId>,
    pub pending_branch_switch: Option<PendingBranchTarget>,
    pub pending_repo_switch: Option<String>,
    pub handshake_scope_nonce: Option<u64>,
}

pub fn accepts_write_ready(input: WriteReadyScopeInput<'_>) -> bool {
    logic::switches_are_idle(
        input.pending_branch_switch.as_ref(),
        input.pending_repo_switch.as_deref(),
    ) && input.handshake_scope_nonce == Some(input.scope_nonce)
        && peer_branch_matches_scope(
            &input.branch,
            input.active_branch.clone(),
            input.pending_branch_switch,
        )
        && input.active_branch.is_none()
        && input.current_repo_id.as_deref() == Some(input.repo_id)
}

pub fn accepts_write_ready_message(
    repo_id: &str,
    branch: &Option<PeerId>,
    scope_nonce: u64,
    signals: CoreSignals,
) -> bool {
    accepts_write_ready(WriteReadyScopeInput {
        repo_id,
        branch: branch.clone(),
        scope_nonce,
        current_repo_id: signals.current_repo_id.get_untracked(),
        active_branch: signals.active_branch.get_untracked(),
        pending_branch_switch: signals
            .pending_branch_switch
            .get_untracked()
            .map(|pending| pending.into_target()),
        pending_repo_switch: signals
            .pending_repo_switch
            .get_untracked()
            .map(|pending| pending.expected_name),
        handshake_scope_nonce: signals.handshake_scope_nonce.get_untracked(),
    })
}

pub fn accepts_edit_rejected_message(scope_nonce: Option<u64>, signals: CoreSignals) -> bool {
    logic::accepts_current_scope_message(
        scope_nonce,
        signals.current_scope_nonce.get_untracked(),
        signals
            .pending_branch_switch
            .get_untracked()
            .map(|pending| pending.into_target()),
        signals
            .pending_repo_switch
            .get_untracked()
            .map(|pending| pending.expected_name),
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
            signals
                .pending_branch_switch
                .get_untracked()
                .map(|pending| pending.switch_nonce),
            signals
                .pending_repo_switch
                .get_untracked()
                .map(|pending| pending.switch_nonce),
        );
    }
    logic::accepts_current_scope_message(
        scope_nonce,
        signals.current_scope_nonce.get_untracked(),
        signals
            .pending_branch_switch
            .get_untracked()
            .map(|pending| pending.into_target()),
        signals
            .pending_repo_switch
            .get_untracked()
            .map(|pending| pending.expected_name),
    )
}
