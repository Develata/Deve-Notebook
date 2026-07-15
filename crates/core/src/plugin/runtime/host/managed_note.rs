//! plan_ref:
//!   - 19_plugins#plugin-runtime-boundary
//!   - 03_storage/authority#repo-mutation-publication-gate
//!
//! Narrow host-owned mutation boundary for managed-note plugin writes.

use anyhow::Result;
use std::sync::{Arc, OnceLock};

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

static MANAGED_NOTE_MUTATION_HOST: OnceLock<Arc<dyn ManagedNoteMutationHost>> = OnceLock::new();

pub fn set_managed_note_mutation_host(host: Arc<dyn ManagedNoteMutationHost>) -> Result<()> {
    MANAGED_NOTE_MUTATION_HOST
        .set(host)
        .map_err(|_| anyhow::anyhow!("ManagedNoteMutationHost already set"))
}

pub(super) fn managed_note_mutation_host() -> Result<Arc<dyn ManagedNoteMutationHost>> {
    MANAGED_NOTE_MUTATION_HOST
        .get()
        .cloned()
        .ok_or_else(|| anyhow::anyhow!("ManagedNoteMutationHost not configured"))
}
