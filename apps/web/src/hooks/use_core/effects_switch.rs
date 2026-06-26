//! plan_ref:
//!   - 07_network#web-ws-runtime
//!   - 04_repository#repo-scope-runtime
//!
use deve_core::models::PeerId;
use leptos::prelude::*;

use super::types::{PendingBranchSwitch, RepoSwitchSignals};
mod branch;
mod repo;

#[cfg(test)]
mod branch_tests;
#[cfg(test)]
mod repo_tests;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RepoSwitchOutcome {
    pub accepted: bool,
    pub should_refresh: bool,
}

#[derive(Clone, Copy)]
pub struct BranchSwitchSignals {
    pub pending_branch_switch: ReadSignal<Option<PendingBranchSwitch>>,
    pub set_pending_branch_switch: WriteSignal<Option<PendingBranchSwitch>>,
    pub set_active_branch: WriteSignal<Option<PeerId>>,
}

pub use branch::handle_branch_switched;
pub use repo::handle_repo_switched;
