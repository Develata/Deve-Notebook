//! plan_ref:
//!   - 10_rendering#document-authority-bridge
//!   - 04_repository#tree-projection-contract
//!
use deve_core::models::{DocId, PeerId, RepoId};
use deve_core::tree::TreeDelta;

use super::super::state::CoreSignals;
use super::message_projection::{handle_doc_list, handle_tree_update};

pub fn handle_doc_list_message(
    request_id: Option<String>,
    repo_id: Option<RepoId>,
    branch: Option<PeerId>,
    scope_nonce: Option<u64>,
    docs: Vec<(DocId, String)>,
    signals: CoreSignals,
) {
    handle_doc_list(request_id, repo_id, branch, scope_nonce, docs, signals);
}

pub fn handle_tree_update_message(
    request_id: Option<String>,
    repo_id: Option<RepoId>,
    branch: Option<PeerId>,
    scope_nonce: Option<u64>,
    delta: TreeDelta,
    signals: CoreSignals,
) {
    handle_tree_update(request_id, repo_id, branch, scope_nonce, delta, signals);
}
