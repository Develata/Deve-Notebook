//! plan_ref:
//!   - 19_plugins#plugin-runtime-boundary
//!   - 03_storage/authority#repo-mutation-publication-gate
//!
//! Narrow host-owned authority boundary for local plugin Source Control commits.

use crate::ledger::traits::RepoSelector;
use crate::protocol::ScPathTarget;
use crate::source_control::CommitInfo;
use anyhow::Result;
use std::sync::Arc;

#[derive(Clone, Debug)]
pub struct ManagedSourceControlCommitIntent {
    pub selector: RepoSelector,
    pub message: String,
}

#[derive(Clone, Debug)]
pub struct ManagedSourceControlStageIntent {
    pub selector: RepoSelector,
    pub target: ScPathTarget,
}

pub trait ManagedSourceControlMutationHost: Send + Sync {
    fn stage_source_control(&self, intent: ManagedSourceControlStageIntent) -> Result<()>;

    fn commit_source_control(&self, intent: ManagedSourceControlCommitIntent)
    -> Result<CommitInfo>;
}

pub(super) fn managed_source_control_mutation_host()
-> Result<Arc<dyn ManagedSourceControlMutationHost>> {
    super::managed_context::managed_source_control_mutation_host()
}
