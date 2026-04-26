//! plan_ref:
//!   - 06_repository#repo-scope-runtime
//!
//! Peer merge apply and conflict emission helpers.

use super::errors;
use super::peer_support::resolve_doc_path;
use crate::server::repo_scope::ResolvedRepo;
use crate::server::{AppState, channel::DualChannel};
use deve_core::ledger::merge::ConflictHunk;
use deve_core::models::DocId;
use deve_core::protocol::{MergeConflictAction, ServerMessage};
use deve_core::sync::reconcile;
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
) {
    let entries = match state
        .repo
        .get_local_ops_in_local_repo(&scope.repo_name, doc_id)
    {
        Ok(entries) => entries
            .into_iter()
            .map(|(_, entry)| entry)
            .collect::<Vec<_>>(),
        Err(err) => {
            return errors::classified_failure(
                ch,
                format!("Failed to load local merge state: {}", err),
                scope_nonce,
            );
        }
    };
    let patch = match reconcile::compute_reconcile_patch(&entries, content) {
        Ok(patch) => patch,
        Err(err) => {
            return errors::request_failed(
                ch,
                format!("Failed to diff merged content: {}", err),
                scope_nonce,
            );
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
        return;
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
        return;
    }
    tracing::info!("Merge Success for doc {}", doc_id);
    ch.broadcast(ServerMessage::MergeComplete {
        repo_id: Some(scope.repo_id),
        branch: scope.branch.clone(),
        scope_nonce,
        merged_count: 1,
    });
}

pub(super) fn send_merge_conflict(
    state: &Arc<AppState>,
    ch: &DualChannel,
    scope: &ResolvedRepo,
    payload: MergeConflictPayload,
    scope_nonce: Option<u64>,
) {
    let Some(path) = resolve_doc_path(state, ch, &scope.repo_name, payload.doc_id, scope_nonce)
    else {
        return;
    };
    emit_merge_conflict(ch, scope, path, payload, scope_nonce);
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
mod tests {
    use super::*;
    use crate::server::channel::DualChannel;
    use deve_core::models::PeerId;
    use deve_core::protocol::ServerErrorCode;
    use tokio::sync::{broadcast, mpsc};

    #[tokio::test]
    async fn merge_conflict_emits_typed_payload_before_diff_fallback() {
        let (broadcast_tx, _) = broadcast::channel(4);
        let (unicast_tx, mut unicast_rx) = mpsc::channel(8);
        let ch = DualChannel::new(broadcast_tx, unicast_tx);
        let doc_id = DocId::new();
        let repo_id = uuid::Uuid::new_v4();
        let scope = ResolvedRepo {
            repo_id,
            repo_name: "notes".into(),
            branch: Some(PeerId::new("remote-a")),
        };
        let hunk = ConflictHunk {
            start_line: 1,
            length: 2,
            local_lines: vec!["local".into()],
            remote_lines: vec!["remote".into()],
        };

        emit_merge_conflict(
            &ch,
            &scope,
            "docs/a.md".into(),
            MergeConflictPayload {
                doc_id,
                base: "base".into(),
                local: "local".into(),
                remote: "remote".into(),
                conflicts: vec![hunk.clone()],
            },
            Some(7),
        );

        match unicast_rx.recv().await {
            Some(ServerMessage::MergeConflict {
                repo_id: Some(actual_repo),
                branch,
                scope_nonce,
                doc_id: actual_doc,
                path,
                current_content,
                incoming_content,
                result_content,
                actions,
                conflicts,
            }) => {
                assert_eq!(actual_repo, repo_id);
                assert_eq!(branch.as_ref().map(PeerId::as_str), Some("remote-a"));
                assert_eq!(scope_nonce, Some(7));
                assert_eq!(actual_doc, doc_id);
                assert_eq!(path, "docs/a.md");
                assert_eq!(current_content, "local");
                assert_eq!(incoming_content, "remote");
                assert_eq!(result_content, "base");
                assert_eq!(actions.len(), 3);
                assert_eq!(conflicts, vec![hunk]);
            }
            other => panic!("expected typed MergeConflict first, got {other:?}"),
        }

        match unicast_rx.recv().await {
            Some(ServerMessage::DocDiff {
                request_id: None,
                repo_id: Some(actual_repo),
                branch,
                scope_nonce,
                path,
                old_content,
                new_content,
            }) => {
                assert_eq!(actual_repo, repo_id);
                assert_eq!(branch.as_ref().map(PeerId::as_str), Some("remote-a"));
                assert_eq!(scope_nonce, Some(7));
                assert_eq!(path, "docs/a.md");
                assert_eq!(old_content, "local");
                assert_eq!(new_content, "remote");
            }
            other => panic!("expected DocDiff fallback second, got {other:?}"),
        }

        match unicast_rx.recv().await {
            Some(ServerMessage::ProtocolError {
                error, scope_nonce, ..
            }) => {
                assert_eq!(error.code, ServerErrorCode::StorageConflict);
                assert_eq!(scope_nonce, Some(7));
            }
            other => panic!("expected StorageConflict third, got {other:?}"),
        }
    }
}
