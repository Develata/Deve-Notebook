use deve_core::models::{DocId, Op};
use std::collections::HashMap;

#[derive(Clone, Debug)]
pub struct PendingLocalEdit {
    pub client_op_id: u64,
    pub op: Op,
}

pub type PendingLocalEdits = HashMap<DocId, Vec<PendingLocalEdit>>;

pub fn push_pending_edit(
    pending: &mut PendingLocalEdits,
    doc_id: DocId,
    client_op_id: u64,
    op: Op,
) {
    pending
        .entry(doc_id)
        .or_default()
        .push(PendingLocalEdit { client_op_id, op });
}

pub fn ack_pending_edit(pending: &mut PendingLocalEdits, doc_id: DocId, client_op_id: u64) -> bool {
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

pub fn cloned_ops_for_doc(pending: &PendingLocalEdits, doc_id: DocId) -> Vec<Op> {
    pending
        .get(&doc_id)
        .into_iter()
        .flatten()
        .map(|edit| edit.op.clone())
        .collect()
}
