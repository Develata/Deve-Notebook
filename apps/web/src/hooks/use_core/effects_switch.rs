//! plan_ref:
//!   - 05_network#web-ws-runtime
//!   - 06_repository#repo-scope-runtime
//!
use deve_core::models::PeerId;
use leptos::prelude::*;

use super::types::{PendingBranchTarget, RepoSwitchSignals};
#[path = "effects_switch_branch.rs"]
mod branch;
#[path = "effects_switch_repo.rs"]
mod repo;

#[cfg(test)]
#[path = "effects_switch_branch_test.rs"]
mod branch_tests;
#[cfg(test)]
#[path = "effects_switch_repo_test.rs"]
mod repo_tests;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RepoSwitchOutcome {
    pub accepted: bool,
    pub should_refresh: bool,
}

#[derive(Clone, Copy)]
pub struct BranchSwitchSignals {
    pub pending_branch_switch: ReadSignal<Option<PendingBranchTarget>>,
    pub pending_branch_switch_nonce: ReadSignal<Option<u64>>,
    pub set_pending_branch_switch: WriteSignal<Option<PendingBranchTarget>>,
    pub set_pending_branch_switch_nonce: WriteSignal<Option<u64>>,
    pub set_active_branch: WriteSignal<Option<PeerId>>,
}

pub use branch::handle_branch_switched;
pub use repo::handle_repo_switched;
