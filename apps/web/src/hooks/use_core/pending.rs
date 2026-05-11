//! plan_ref:
//!   - 03_rendering#document-authority-bridge
//!   - 16_web_thin_client_ledger#web-edit-intent
//!
use deve_core::models::{DocId, Op, RepoId};
use std::collections::HashMap;

mod history;
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PendingScope {
    pub repo_id: RepoId,
    pub scope_nonce: u64,
}

impl PendingScope {
    pub fn from_repo_id_str(repo_id: Option<&str>, scope_nonce: u64) -> Option<Self> {
        repo_id.and_then(|repo_id| {
            repo_id.parse::<RepoId>().ok().map(|repo_id| Self {
                repo_id,
                scope_nonce,
            })
        })
    }
}

pub type PendingLocalEdits = HashMap<DocId, Vec<PendingLocalEdit>>;
pub use history::reconcile_with_history;
pub use ops::{
    clear_pending_edit_and_check_current_doc_empty, cloned_ops_for_doc,
    cloned_ops_for_doc_in_scope, cloned_pending_edits_for_doc,
    cloned_pending_edits_for_doc_in_scope, has_pending_edit, has_pending_edits_for_doc,
    has_pending_edits_for_doc_in_scope, pending_count_for_doc, pending_count_for_doc_in_scope,
    push_pending_edit,
};
#[cfg(test)]
mod ops_tests;
