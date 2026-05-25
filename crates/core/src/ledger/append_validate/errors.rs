//! plan_ref:
//!   - 03_storage/authority#facts-partition
//!   - 04_repository#tree-projection-contract
//!
use crate::models::{DocId, LedgerEntry, StructureOp};
use crate::state::{InvalidContentOp, describe_invalid_content_op};
use anyhow::anyhow;

pub(super) fn reject_missing_doc_id(entry: &LedgerEntry, repo_scope: &str) -> anyhow::Error {
    tracing::warn!(repo_scope, doc_id = "<missing>", peer_id = %entry.peer_id, seq = entry.seq, issue = "content op missing doc id", "Rejecting invalid ledger append");
    anyhow!("Content op missing doc id")
}

pub(super) fn reject_invalid_content(
    doc_id: DocId,
    entry: &LedgerEntry,
    issue: &InvalidContentOp,
    existing_history_invalid: bool,
    repo_scope: &str,
) -> anyhow::Error {
    let issue_text = describe_invalid_content_op(issue);
    tracing::warn!(repo_scope, doc_id = %doc_id, peer_id = %entry.peer_id, seq = entry.seq, issue = %issue_text, existing_history_invalid, "Rejecting invalid ledger append");
    if existing_history_invalid {
        anyhow!(
            "Refusing to append content op for {}: existing history invalid: {}",
            doc_id,
            issue_text
        )
    } else {
        anyhow!(
            "Refusing to append content op for {}: {}",
            doc_id,
            issue_text
        )
    }
}

pub(super) fn reject_invalid_structure(
    op: &StructureOp,
    issue: &str,
    repo_scope: &str,
) -> anyhow::Error {
    tracing::warn!(repo_scope, node_id = %op.node_id(), doc_id = ?op.doc_id(), issue, "Rejecting invalid structure append");
    anyhow!(
        "Refusing to append structure op for {}: {}",
        op.node_id(),
        issue
    )
}
