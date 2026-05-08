//! plan_ref:
//!   - 03_rendering#document-authority-bridge
//!   - 16_web_thin_client_ledger#web-edit-intent
//!
use deve_core::models::{DocId, Op, RepoId};
use std::collections::HashMap;

#[path = "pending_history.rs"]
mod history;
#[path = "pending_ops.rs"]
mod ops;
#[cfg(test)]
mod tests;

#[derive(Clone, Debug)]
pub struct PendingLocalEditInput {
    pub repo_id: RepoId,
    pub doc_id: DocId,
    pub scope_nonce: u64,
    pub client_id: u64,
    pub client_op_id: u64,
    pub base_version: u64,
    pub op: Op,
}

#[derive(Clone, Debug)]
pub struct PendingLocalEdit {
    pub repo_id: RepoId,
    pub doc_id: DocId,
    pub scope_nonce: u64,
    pub client_id: u64,
    pub client_op_id: u64,
    pub created_at_ms: i64,
    pub base_version: u64,
    pub op_marker: String,
    pub op: Op,
}

pub type PendingLocalEdits = HashMap<DocId, Vec<PendingLocalEdit>>;
pub use history::reconcile_with_history;
pub use ops::{
    clear_pending_edit_and_check_current_doc_empty, cloned_ops_for_doc,
    cloned_pending_edits_for_doc, has_pending_edits_for_doc, pending_count_for_doc,
    push_pending_edit,
};
#[cfg(test)]
#[path = "pending_ops_test.rs"]
mod ops_tests;
