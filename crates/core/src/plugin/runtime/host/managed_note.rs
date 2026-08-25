//! plan_ref:
//!   - 19_plugins#plugin-runtime-boundary
//!   - 03_storage/authority#repo-mutation-publication-gate
//!
//! Narrow host-owned mutation boundary for managed-note plugin writes.

use anyhow::Result;
use std::sync::Arc;

/// Business intent emitted by the Rhai host after capability and managed-path checks.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ManagedNoteWriteIntent {
    pub repo_name: String,
    pub repo_path: String,
    pub content: String,
}

/// Host adapter that owns repository serialization, authority mutation, projection
/// persistence, and recovery publication for a managed-note write.
pub trait ManagedNoteMutationHost: Send + Sync {
    fn write_managed_note(&self, intent: ManagedNoteWriteIntent) -> Result<()>;
}

pub(super) fn managed_note_mutation_host() -> Result<Arc<dyn ManagedNoteMutationHost>> {
    super::managed_context::managed_note_mutation_host()
}
