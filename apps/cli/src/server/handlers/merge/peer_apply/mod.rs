//! plan_ref:
//!   - 04_repository#repo-scope-runtime
//!
//! Peer merge apply and conflict emission helpers.

use super::errors;
use super::peer_support::resolve_doc_path;
use crate::server::repo_mutation::{MountedRepoAdmission, MutationExecution, MutationPublication};
use crate::server::repo_scope::{ResolvedRepo, ensure_resolved_local_repo_writable};
use crate::server::{AppState, channel::DualChannel, session::WsSession};
use deve_core::ledger::merge::ConflictHunk;
use deve_core::ledger::merge::MergePreflight;
use deve_core::models::{DocId, MergeResolution};
use deve_core::protocol::MergeConflictAction;
use std::sync::Arc;

pub(super) struct MergeConflictPayload {
    pub(super) doc_id: DocId,
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

pub(super) struct MergeWriteRequest<'a> {
    pub(super) scope: &'a ResolvedRepo,
    pub(super) admission: MountedRepoAdmission,
    pub(super) preflight: &'a MergePreflight,
    pub(super) content: &'a str,
    pub(super) resolution: MergeResolution,
    pub(super) scope_nonce: Option<u64>,
}

pub(super) async fn write_merged_content(
    state: &Arc<AppState>,
    ch: &DualChannel,
    request: MergeWriteRequest<'_>,
) -> MergeWriteOutcome {
    let MergeWriteRequest {
        scope,
        admission,
        preflight,
        content,
        resolution,
        scope_nonce,
    } = request;
    if let Err(error) = ensure_resolved_local_repo_writable(state, scope) {
        ch.send_protocol_error_with_scope_nonce(error, scope_nonce);
        return MergeWriteOutcome::CommitFailed;
    }
    let execution = state
        .repo_mutation_gate()
        .execute_admitted_mounted_repo(admission, &state.tx, || {
            let scope =
                match crate::server::repo_mutation::revalidate_writable_resolved_repo(state, scope)
                {
                    Ok(scope) => scope,
                    Err(error) => return MutationExecution::not_committed(error),
                };
            let outcome = match state.repo.commit_peer_merge_in_local_repo(
                &scope.repo_name,
                preflight,
                content,
                resolution,
            ) {
                Ok(outcome) => outcome,
                Err(error) => return MutationExecution::not_committed(error),
            };
            let merged_count = u32::from(outcome.content_changed);
            let publication = MutationPublication::MergeComplete {
                repo_id: scope.repo_id,
                branch: scope.branch.clone(),
                scope_nonce,
                merged_count,
                recovery: MutationPublication::merge_recovery(scope.repo_id, preflight.doc_id()),
            };
            if outcome.content_changed
                && let Err(error) = state
                    .sync_manager
                    .persist_doc_in_local_repo(&scope.repo_name, preflight.doc_id())
            {
                return MutationExecution::projection_degraded(outcome, error, publication);
            }
            MutationExecution::committed(outcome, publication)
        })
        .await;
    match execution {
        Ok(MutationExecution::Committed { .. }) => {
            tracing::info!("Merge Success for doc {}", preflight.doc_id());
            MergeWriteOutcome::Committed
        }
        Ok(MutationExecution::NotCommitted(err)) => {
            errors::storage_persist_failed(
                ch,
                format!("Failed to append merged content and checkpoint: {}", err),
                scope_nonce,
            );
            MergeWriteOutcome::CommitFailed
        }
        Ok(MutationExecution::ProjectionDegraded { error, .. })
        | Ok(MutationExecution::CommittedPartial { error, .. }) => {
            errors::storage_persist_failed(
                ch,
                format!("Failed to persist merged content: {error}"),
                scope_nonce,
            );
            MergeWriteOutcome::CommittedWritebackFailed
        }
        Err(error) => {
            errors::server_error(ch, error.server_error(), scope_nonce);
            MergeWriteOutcome::CommitFailed
        }
    }
}

pub(super) fn send_merge_conflict(
    state: &Arc<AppState>,
    ch: &DualChannel,
    session: &mut WsSession,
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
    emit_merge_conflict(
        state,
        ch,
        session,
        message_scope,
        path,
        payload,
        scope_nonce,
    );
    true
}

fn emit_merge_conflict(
    state: &Arc<AppState>,
    ch: &DualChannel,
    session: &mut WsSession,
    scope: &ResolvedRepo,
    path: String,
    payload: MergeConflictPayload,
    scope_nonce: Option<u64>,
) {
    tracing::warn!("Merge Conflict detected for doc {}", payload.doc_id);
    let actions = vec![
        MergeConflictAction::AcceptCurrent,
        MergeConflictAction::AcceptIncoming,
        MergeConflictAction::AcceptBoth,
    ];
    let request_id = format!("merge-{}", payload.doc_id);
    let result_content = default_accept_both(&payload.local, &payload.remote);
    let ticket = session.diff_projection_jobs.begin_fixed(
        request_id,
        scope.repo_id,
        scope.branch.clone(),
        deve_core::protocol::ScopeNonce::new(scope_nonce.unwrap_or_default()),
    );
    state.diff_projection_executor().spawn(
        ticket,
        payload.local,
        payload.remote,
        crate::server::diff_projection::DiffJobResponse::Merge {
            doc_id: payload.doc_id,
            path,
            result_content,
            actions,
            conflicts: payload.conflicts,
        },
        ch.clone(),
    );
    errors::storage_conflict(
        ch,
        "Merge Conflict detected. Computing Diff View.",
        scope_nonce,
    );
}

pub(super) fn default_accept_both(current: &str, incoming: &str) -> String {
    if current.is_empty() || incoming.is_empty() || current.ends_with('\n') {
        format!("{current}{incoming}")
    } else {
        format!("{current}\n{incoming}")
    }
}

#[cfg(test)]
mod tests;
