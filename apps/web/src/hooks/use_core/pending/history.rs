//! plan_ref:
//!   - 10_rendering#document-authority-bridge
//!   - 09_web_thin_client_ledger#web-edit-intent
//!
use deve_core::models::DocId;
use deve_core::protocol::{ClientOrigin, ConfirmedOp};

use super::{PendingLocalEdit, PendingLocalEdits};

pub fn reconcile_with_history(
    pending: &mut PendingLocalEdits,
    doc_id: DocId,
    history: &[ConfirmedOp],
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

fn unresolved_edits(edits: &[PendingLocalEdit], history: &[ConfirmedOp]) -> Vec<PendingLocalEdit> {
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

fn find_history_match(edit: &PendingLocalEdit, history: &[ConfirmedOp]) -> Option<usize> {
    history
        .iter()
        .enumerate()
        .rev()
        .find(|(_, entry)| {
            entry.seq > edit.base_version
                && entry.origin
                    == Some(ClientOrigin {
                        client_id: edit.client_id,
                        client_op_id: edit.client_op_id,
                    })
        })
        .map(|(idx, _)| idx)
}
