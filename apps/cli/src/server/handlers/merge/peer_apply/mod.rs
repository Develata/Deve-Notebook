//! plan_ref:
//!   - 04_repository#repo-scope-runtime
//!
//! Peer merge apply and conflict emission helpers.

use super::errors;
use super::peer_support::resolve_doc_path;
use crate::server::repo_scope::{ResolvedRepo, ensure_resolved_local_repo_writable};
use crate::server::{AppState, channel::DualChannel};
use deve_core::ledger::merge::ConflictHunk;
use deve_core::ledger::merge::MergePreflight;
use deve_core::models::{DocId, MergeResolution};
use deve_core::protocol::{MergeConflictAction, ServerMessage};
use std::sync::Arc;

pub(super) struct MergeConflictPayload {
    pub(super) doc_id: DocId,
    pub(super) base: String,
    pub(super) local: String,
    pub(super) remote: String,
    pub(super) conflicts: Vec<ConflictHunk>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum MergeWriteOutcome {
    Committed,
    CommitFailed,
    CommittedWritebackFailed,
}

pub(super) fn write_merged_content(
    state: &Arc<AppState>,
    ch: &DualChannel,
    scope: &ResolvedRepo,
    preflight: &MergePreflight,
    content: &str,
    resolution: MergeResolution,
    scope_nonce: Option<u64>,
) -> MergeWriteOutcome {
    if let Err(error) = ensure_resolved_local_repo_writable(state, scope) {
        ch.send_protocol_error_with_scope_nonce(error, scope_nonce);
        return MergeWriteOutcome::CommitFailed;
    }
    let outcome = match state.repo.commit_peer_merge_in_local_repo(
        &scope.repo_name,
        preflight,
        content,
        resolution,
    ) {
        Ok(outcome) => outcome,
        Err(err) => {
            errors::storage_persist_failed(
                ch,
                format!("Failed to append merged content and checkpoint: {}", err),
                scope_nonce,
            );
            return MergeWriteOutcome::CommitFailed;
        }
    };
    if outcome.content_changed
        && let Err(err) = state
            .sync_manager
            .persist_doc_in_local_repo(&scope.repo_name, preflight.doc_id())
    {
        errors::storage_persist_failed(
            ch,
            format!("Failed to persist merged content: {}", err),
            scope_nonce,
        );
        return MergeWriteOutcome::CommittedWritebackFailed;
    }
    tracing::info!("Merge Success for doc {}", preflight.doc_id());
    broadcast_merge_complete(ch, scope, u32::from(outcome.content_changed), scope_nonce);
    MergeWriteOutcome::Committed
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
    path_scope: &ResolvedRepo,
    message_scope: &ResolvedRepo,
    payload: MergeConflictPayload,
    scope_nonce: Option<u64>,
) -> bool {
    let Some(path) = resolve_doc_path(
        state,
        ch,
        &path_scope.repo_name,
        payload.doc_id,
        scope_nonce,
    ) else {
        return false;
    };
    emit_merge_conflict(ch, message_scope, path, payload, scope_nonce);
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
