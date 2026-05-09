//! plan_ref:
//!   - 06_repository#repo-scope-runtime
//!
//! Peer merge apply and conflict emission helpers.

use super::errors;
use super::peer_support::resolve_doc_path;
use crate::server::repo_scope::ResolvedRepo;
use crate::server::{AppState, channel::DualChannel};
use deve_core::ledger::merge::ConflictHunk;
use deve_core::ledger::reconcile;
use deve_core::models::DocId;
use deve_core::protocol::{MergeConflictAction, ServerMessage};
use std::sync::Arc;

pub(super) struct MergeConflictPayload {
    pub(super) doc_id: DocId,
    pub(super) base: String,
    pub(super) local: String,
    pub(super) remote: String,
    pub(super) conflicts: Vec<ConflictHunk>,
}

pub(super) fn write_merged_content(
    state: &Arc<AppState>,
    ch: &DualChannel,
    scope: &ResolvedRepo,
    doc_id: DocId,
    content: &str,
    scope_nonce: Option<u64>,
) -> bool {
    let entries = match state
        .repo
        .get_local_ops_in_local_repo(&scope.repo_name, doc_id)
    {
        Ok(entries) => entries
            .into_iter()
            .map(|(_, entry)| entry)
            .collect::<Vec<_>>(),
        Err(err) => {
            errors::classified_failure(
                ch,
                format!("Failed to load local merge state: {}", err),
                scope_nonce,
            );
            return false;
        }
    };
    let patch = match reconcile::compute_reconcile_patch(&entries, content) {
        Ok(patch) => patch,
        Err(err) => {
            errors::request_failed(
                ch,
                format!("Failed to diff merged content: {}", err),
                scope_nonce,
            );
            return false;
        }
    };
    if let Err(err) = reconcile::append_patch_in_local_repo(
        &state.repo,
        &scope.repo_name,
        doc_id,
        "merge",
        &patch,
    ) {
        errors::storage_persist_failed(
            ch,
            format!("Failed to append merged content: {}", err),
            scope_nonce,
        );
        return false;
    }
    if let Err(err) = state
        .sync_manager
        .persist_doc_in_local_repo(&scope.repo_name, doc_id)
    {
        errors::storage_persist_failed(
            ch,
            format!("Failed to persist merged content: {}", err),
            scope_nonce,
        );
        return false;
    }
    tracing::info!("Merge Success for doc {}", doc_id);
    broadcast_merge_complete(ch, scope, 1, scope_nonce);
    true
}

pub(super) fn broadcast_merge_complete(
    ch: &DualChannel,
    scope: &ResolvedRepo,
    merged_count: u32,
    scope_nonce: Option<u64>,
) {
    ch.broadcast(ServerMessage::MergeComplete {
        repo_id: Some(scope.repo_id),
        branch: scope.branch.clone(),
        scope_nonce,
        merged_count,
    });
}

pub(super) fn send_merge_conflict(
    state: &Arc<AppState>,
    ch: &DualChannel,
    scope: &ResolvedRepo,
    payload: MergeConflictPayload,
    scope_nonce: Option<u64>,
) -> bool {
    let Some(path) = resolve_doc_path(state, ch, &scope.repo_name, payload.doc_id, scope_nonce)
    else {
        return false;
    };
    emit_merge_conflict(ch, scope, path, payload, scope_nonce);
    true
}

fn emit_merge_conflict(
    ch: &DualChannel,
    scope: &ResolvedRepo,
    path: String,
    payload: MergeConflictPayload,
    scope_nonce: Option<u64>,
) {
    tracing::warn!("Merge Conflict detected for doc {}", payload.doc_id);
    ch.unicast(ServerMessage::MergeConflict {
        repo_id: Some(scope.repo_id),
        branch: scope.branch.clone(),
        scope_nonce,
        doc_id: payload.doc_id,
        path: path.clone(),
        current_content: payload.local.clone(),
        incoming_content: payload.remote.clone(),
        result_content: payload.base,
        actions: vec![
            MergeConflictAction::AcceptCurrent,
            MergeConflictAction::AcceptIncoming,
            MergeConflictAction::AcceptBoth,
        ],
        conflicts: payload.conflicts,
    });
    ch.unicast(ServerMessage::DocDiff {
        request_id: None,
        repo_id: Some(scope.repo_id),
        branch: scope.branch.clone(),
        scope_nonce,
        doc_id: Some(payload.doc_id),
        path,
        old_content: payload.local,
        new_content: payload.remote,
    });
    errors::storage_conflict(
        ch,
        "Merge Conflict detected. Showing Diff View.",
        scope_nonce,
    );
}

#[cfg(test)]
mod tests;
