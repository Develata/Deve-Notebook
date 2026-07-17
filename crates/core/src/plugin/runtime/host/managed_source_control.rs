//! plan_ref:
//!   - 19_plugins#plugin-runtime-boundary
//!   - 03_storage/authority#repo-mutation-publication-gate
//!
//! Narrow host-owned authority boundary for local plugin Source Control commits.

use crate::ledger::traits::RepoSelector;
use crate::protocol::ScPathTarget;
use crate::source_control::CommitInfo;
use anyhow::Result;
use std::sync::{Arc, OnceLock};

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

static MANAGED_SOURCE_CONTROL_MUTATION_HOST: OnceLock<Arc<dyn ManagedSourceControlMutationHost>> =
    OnceLock::new();

pub fn set_managed_source_control_mutation_host(
    host: Arc<dyn ManagedSourceControlMutationHost>,
) -> Result<()> {
    MANAGED_SOURCE_CONTROL_MUTATION_HOST
        .set(host)
        .map_err(|_| anyhow::anyhow!("ManagedSourceControlMutationHost already set"))
}

pub(super) fn managed_source_control_mutation_host()
-> Result<Arc<dyn ManagedSourceControlMutationHost>> {
    MANAGED_SOURCE_CONTROL_MUTATION_HOST
        .get()
        .cloned()
        .ok_or_else(|| anyhow::anyhow!("ManagedSourceControlMutationHost not configured"))
}
