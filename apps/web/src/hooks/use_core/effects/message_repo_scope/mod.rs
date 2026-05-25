//! plan_ref:
//!   - 07_network#web-ws-runtime
//!   - 04_repository#repo-scope-runtime
//!
use crate::hooks::use_core::PendingBranchTarget;
use crate::hooks::use_core::state::CoreSignals;
use deve_core::models::PeerId;
use leptos::prelude::*;

mod accept;
mod logic;
#[cfg(test)]
pub use self::accept::{WriteReadyScopeInput, accepts_write_ready};
pub use self::accept::{
    accepts_edit_rejected_message, accepts_protocol_error_message, accepts_write_ready_message,
};

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

#[cfg(test)]
mod tests;
