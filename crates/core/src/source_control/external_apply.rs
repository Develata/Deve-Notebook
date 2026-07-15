//! plan_ref:
//!   - 05_diff_logic#source-control-runtime
//!   - 03_storage/authority#repo-mutation-publication-gate
//!
//! Transaction receipt for an External Changes apply operation.

use crate::models::{DocId, GlobalSeq, RepoId};
use serde::{Deserialize, Serialize};

#[cfg(not(target_arch = "wasm32"))]
use crate::models::{FileNodeId, Op};
#[cfg(not(target_arch = "wasm32"))]
use crate::source_control::staging::StagedEntry;

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
    pub(crate) repo_id: RepoId,
    pub(crate) expected_ledger_head: u64,
    pub(crate) staged_snapshot: Vec<(String, StagedEntry)>,
    pub(crate) targets: Vec<PreparedExternalTarget>,
    pub(crate) changed_paths: Vec<String>,
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) struct PreparedUpsert {
    pub(crate) path: String,
    pub(crate) doc_id: DocId,
    pub(crate) content_ops: Vec<Op>,
    pub(crate) inode: Option<FileNodeId>,
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) enum PreparedExternalTarget {
    Upsert(PreparedUpsert),
    Delete { path: String, doc_id: Option<DocId> },
}
