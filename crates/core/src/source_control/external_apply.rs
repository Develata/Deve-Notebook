//! plan_ref:
//!   - 05_diff_logic#source-control-runtime
//!   - 03_storage/authority#repo-mutation-publication-gate
//!
//! Transaction receipt for an External Changes apply operation.

use crate::models::{DocId, GlobalSeq, RepoId};
use serde::{Deserialize, Serialize};

#[cfg(not(target_arch = "wasm32"))]
use crate::ledger::manager::prepared_change_batch::PreparedLedgerChangeBatch;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExternalApplyReceipt {
    pub repo_id: RepoId,
    pub authority_head: GlobalSeq,
    pub affected_docs: Vec<DocId>,
    pub applied_target_count: u32,
}

#[cfg(not(target_arch = "wasm32"))]
#[derive(Clone, Debug)]
pub struct ExternalApplyOutcome {
    pub receipt: ExternalApplyReceipt,
    pub changed_paths: Vec<String>,
}

/// Opaque, read-only preflight result. Expensive workspace reads, content
/// reconstruction, and patch computation happen before the server acquires a
/// repository mutation permit; the commit phase exact-compares this snapshot.
#[cfg(not(target_arch = "wasm32"))]
pub struct PreparedExternalApply {
    batch: PreparedLedgerChangeBatch,
}

#[cfg(not(target_arch = "wasm32"))]
impl PreparedExternalApply {
    pub(crate) fn from_batch(batch: PreparedLedgerChangeBatch) -> Self {
        Self { batch }
    }

    pub(crate) fn into_batch(self) -> PreparedLedgerChangeBatch {
        self.batch
    }
}
