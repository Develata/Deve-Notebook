use deve_core::models::{DocId, Op};
use std::collections::HashMap;

#[cfg(test)]
mod tests;

#[derive(Clone, Debug)]
pub struct PendingLocalEdit {
    pub client_id: u64,
    pub client_op_id: u64,
    pub base_version: u64,
    pub op: Op,
}

pub type PendingLocalEdits = HashMap<DocId, Vec<PendingLocalEdit>>;

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

pub fn cloned_pending_edits_for_doc(
    pending: &PendingLocalEdits,
    doc_id: DocId,
) -> Vec<PendingLocalEdit> {
    pending.get(&doc_id).cloned().unwrap_or_default()
}

pub fn reconcile_with_history(
    pending: &mut PendingLocalEdits,
    doc_id: DocId,
    history: &[(u64, Op)],
) -> usize {
    let (removed, empty) = {
        let Some(edits) = pending.get_mut(&doc_id) else {
            return 0;
        };
        let kept = unresolved_edits(edits, history);
        let removed = edits.len().saturating_sub(kept.len());
        *edits = kept;
        (removed, edits.is_empty())
    };
    if empty {
        pending.remove(&doc_id);
    }
    removed
}

fn unresolved_edits(edits: &[PendingLocalEdit], history: &[(u64, Op)]) -> Vec<PendingLocalEdit> {
    let mut cursor = history.len();
    let mut kept = Vec::new();

    for edit in edits.iter().rev() {
        if let Some(next_cursor) = find_history_match(edit, &history[..cursor]) {
            cursor = next_cursor;
        } else {
            kept.push(edit.clone());
        }
    }

    kept.reverse();
    kept
}

fn find_history_match(edit: &PendingLocalEdit, history: &[(u64, Op)]) -> Option<usize> {
    history
        .iter()
        .enumerate()
        .rev()
        .find(|(_, (seq, op))| *seq > edit.base_version && *op == edit.op)
        .map(|(idx, _)| idx)
}
