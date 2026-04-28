//! plan_ref:
//!   - 03_rendering#document-authority-bridge
//!   - 16_web_thin_client_ledger#web-edit-intent
//!
use deve_core::models::{DocId, Op};

use super::{PendingLocalEdit, PendingLocalEdits};

pub fn push_pending_edit(
    pending: &mut PendingLocalEdits,
    doc_id: DocId,
    client_id: u64,
    client_op_id: u64,
    base_version: u64,
    op: Op,
) {
    pending.entry(doc_id).or_default().push(PendingLocalEdit {
        client_id,
        client_op_id,
        base_version,
        op,
    });
}

fn ack_pending_edit(pending: &mut PendingLocalEdits, doc_id: DocId, client_op_id: u64) -> bool {
    let (changed, empty) = {
        let Some(edits) = pending.get_mut(&doc_id) else {
            return false;
        };
        let before = edits.len();
        edits.retain(|edit| edit.client_op_id != client_op_id);
        (edits.len() != before, edits.is_empty())
    };
    if empty {
        pending.remove(&doc_id);
    }
    changed
}

pub fn clear_pending_edit_and_check_current_doc_empty(
    pending: &mut PendingLocalEdits,
    current_doc: Option<DocId>,
    doc_id: DocId,
    client_op_id: u64,
) -> bool {
    ack_pending_edit(pending, doc_id, client_op_id)
        && current_doc == Some(doc_id)
        && !pending.contains_key(&doc_id)
}

pub fn cloned_ops_for_doc(pending: &PendingLocalEdits, doc_id: DocId) -> Vec<Op> {
    pending
        .get(&doc_id)
        .into_iter()
        .flatten()
        .map(|edit| edit.op.clone())
        .collect()
}

pub fn cloned_pending_edits_for_doc(
    pending: &PendingLocalEdits,
    doc_id: DocId,
) -> Vec<PendingLocalEdit> {
    pending.get(&doc_id).cloned().unwrap_or_default()
}
