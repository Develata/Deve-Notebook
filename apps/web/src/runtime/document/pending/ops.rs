//! plan_ref:
//!   - 10_rendering#document-authority-bridge
//!   - 09_web_thin_client_ledger#web-edit-intent
//!
use deve_core::models::{DocId, Op, RepoId};

use super::{PendingLocalEdit, PendingLocalEditInput, PendingLocalEdits, PendingScope};

pub fn push_pending_edit(pending: &mut PendingLocalEdits, input: PendingLocalEditInput) {
    let PendingLocalEditInput {
        repo_id,
        doc_id,
        scope_nonce,
        client_id,
        client_op_id,
        base_version,
        op,
    } = input;
    let op_marker = op_marker(&op);
    pending.entry(doc_id).or_default().push(PendingLocalEdit {
        repo_id,
        client_id,
        doc_id,
        scope_nonce,
        client_op_id,
        created_at_ms: now_millis(),
        base_version,
        op_marker,
        op,
    });
}

pub fn rebind_pending_scope(
    pending: &mut PendingLocalEdits,
    repo_id: RepoId,
    previous_scope_nonce: u64,
    next_scope_nonce: u64,
) -> usize {
    if previous_scope_nonce == next_scope_nonce {
        return 0;
    }
    let mut rebound = 0;
    for edit in pending.values_mut().flatten() {
        if edit.repo_id == repo_id && edit.scope_nonce == previous_scope_nonce {
            edit.scope_nonce = next_scope_nonce;
            rebound += 1;
        }
    }
    rebound
}

fn ack_pending_edit(
    pending: &mut PendingLocalEdits,
    repo_id: Option<RepoId>,
    scope_nonce: Option<u64>,
    doc_id: DocId,
    client_op_id: u64,
) -> bool {
    let (changed, empty) = {
        let Some(edits) = pending.get_mut(&doc_id) else {
            return false;
        };
        let before = edits.len();
        edits.retain(|edit| {
            edit.client_op_id != client_op_id
                || repo_id.is_some_and(|repo_id| edit.repo_id != repo_id)
                || scope_nonce.is_some_and(|scope_nonce| edit.scope_nonce != scope_nonce)
        });
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
    repo_id: Option<RepoId>,
    scope_nonce: Option<u64>,
    doc_id: DocId,
    client_op_id: u64,
) -> bool {
    let changed = ack_pending_edit(pending, repo_id, scope_nonce, doc_id, client_op_id);
    changed
        && current_doc == Some(doc_id)
        && pending_count_for_doc_filtered(pending, doc_id, repo_id, scope_nonce) == 0
}

#[cfg(test)]
pub fn cloned_ops_for_doc(pending: &PendingLocalEdits, doc_id: DocId) -> Vec<Op> {
    cloned_ops_for_doc_filtered(pending, doc_id, None, None)
}

pub fn cloned_ops_for_doc_in_scope(
    pending: &PendingLocalEdits,
    doc_id: DocId,
    scope: PendingScope,
) -> Vec<Op> {
    cloned_ops_for_doc_filtered(
        pending,
        doc_id,
        Some(scope.repo_id),
        Some(scope.scope_nonce),
    )
}

fn cloned_ops_for_doc_filtered(
    pending: &PendingLocalEdits,
    doc_id: DocId,
    repo_id: Option<RepoId>,
    scope_nonce: Option<u64>,
) -> Vec<Op> {
    pending
        .get(&doc_id)
        .into_iter()
        .flatten()
        .filter(|edit| overlay_row_matches_filters(edit, doc_id, repo_id, scope_nonce))
        .map(|edit| edit.op.clone())
        .collect()
}

#[cfg(test)]
pub fn cloned_pending_edits_for_doc(
    pending: &PendingLocalEdits,
    doc_id: DocId,
) -> Vec<PendingLocalEdit> {
    cloned_pending_edits_for_doc_filtered(pending, doc_id, None, None)
}

pub fn cloned_pending_edits_for_doc_in_scope(
    pending: &PendingLocalEdits,
    doc_id: DocId,
    scope: PendingScope,
) -> Vec<PendingLocalEdit> {
    cloned_pending_edits_for_doc_filtered(
        pending,
        doc_id,
        Some(scope.repo_id),
        Some(scope.scope_nonce),
    )
}

fn cloned_pending_edits_for_doc_filtered(
    pending: &PendingLocalEdits,
    doc_id: DocId,
    repo_id: Option<RepoId>,
    scope_nonce: Option<u64>,
) -> Vec<PendingLocalEdit> {
    pending
        .get(&doc_id)
        .into_iter()
        .flatten()
        .filter(|edit| overlay_row_matches_filters(edit, doc_id, repo_id, scope_nonce))
        .cloned()
        .collect()
}

#[cfg(test)]
pub fn pending_count_for_doc(pending: &PendingLocalEdits, doc_id: DocId) -> usize {
    pending_count_for_doc_filtered(pending, doc_id, None, None)
}

pub fn pending_count_for_doc_in_scope(
    pending: &PendingLocalEdits,
    doc_id: DocId,
    scope: PendingScope,
) -> usize {
    pending_count_for_doc_filtered(
        pending,
        doc_id,
        Some(scope.repo_id),
        Some(scope.scope_nonce),
    )
}

fn pending_count_for_doc_filtered(
    pending: &PendingLocalEdits,
    doc_id: DocId,
    repo_id: Option<RepoId>,
    scope_nonce: Option<u64>,
) -> usize {
    pending
        .get(&doc_id)
        .into_iter()
        .flatten()
        .filter(|edit| overlay_row_matches_filters(edit, doc_id, repo_id, scope_nonce))
        .count()
}

#[cfg(test)]
pub fn has_pending_edits_for_doc(pending: &PendingLocalEdits, doc_id: DocId) -> bool {
    pending_count_for_doc(pending, doc_id) > 0
}

pub fn has_pending_edits_for_doc_in_scope(
    pending: &PendingLocalEdits,
    doc_id: DocId,
    scope: PendingScope,
) -> bool {
    pending_count_for_doc_in_scope(pending, doc_id, scope) > 0
}

pub fn has_pending_edit(
    pending: &PendingLocalEdits,
    repo_id: Option<RepoId>,
    scope_nonce: Option<u64>,
    doc_id: DocId,
    client_op_id: u64,
) -> bool {
    pending.get(&doc_id).into_iter().flatten().any(|edit| {
        edit.client_op_id == client_op_id
            && overlay_row_matches_filters(edit, doc_id, repo_id, scope_nonce)
    })
}

fn op_marker(op: &Op) -> String {
    match op {
        Op::Insert { pos, content } => format!("insert:{pos}:{}", content.len()),
        Op::Delete { pos, len } => format!("delete:{pos}:{len}"),
    }
}

fn overlay_row_matches_doc(edit: &PendingLocalEdit, doc_id: DocId) -> bool {
    edit.doc_id == doc_id && edit.created_at_ms >= 0 && !edit.op_marker.is_empty()
}

fn overlay_row_matches_filters(
    edit: &PendingLocalEdit,
    doc_id: DocId,
    repo_id: Option<RepoId>,
    scope_nonce: Option<u64>,
) -> bool {
    overlay_row_matches_doc(edit, doc_id)
        && repo_id.is_none_or(|repo_id| edit.repo_id == repo_id)
        && scope_nonce.is_none_or(|scope_nonce| edit.scope_nonce == scope_nonce)
}

fn now_millis() -> i64 {
    #[cfg(target_arch = "wasm32")]
    {
        js_sys::Date::now() as i64
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        use std::time::{SystemTime, UNIX_EPOCH};
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_millis() as i64)
            .unwrap_or(0)
    }
}
