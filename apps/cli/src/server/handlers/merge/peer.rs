use crate::server::repo_scope::{
    ResolvedRepo, map_repo_scope_error, resolve_session_repo_and_sync,
};
use crate::server::{AppState, channel::DualChannel, session::WsSession};
use deve_core::ledger::merge::MergeResult;
use deve_core::models::{DocId, PeerId};
use deve_core::protocol::ServerMessage;
use deve_core::sync::reconcile;
use std::sync::Arc;

use super::errors;
use super::peer_support::{resolve_doc_path, resolve_local_merge_scope};

/// Invariants:
/// - 合并目标必须是当前会话解析出的本地 repo。
/// - 远端影子分支内容绝不能写回到其他 repo 的 metadata/path 映射。
pub(super) async fn handle_merge_peer(
    state: &Arc<AppState>,
    ch: &DualChannel,
    session: &mut WsSession,
    peer_id: String,
    doc_id: DocId,
) {
    let scope = match resolve_session_repo_and_sync(state, session) {
        Ok(scope) => scope,
        Err(e) => {
            ch.send_protocol_error(map_repo_scope_error(e));
            return;
        }
    };
    let Some(local_scope) = resolve_local_merge_scope(state, scope, ch) else {
        return;
    };
    let peer_id = PeerId::new(peer_id);
    match state.repo.merge_peer_in_local_repo(
        &local_scope.repo_name,
        &peer_id,
        &local_scope.repo_id,
        doc_id,
    ) {
        Ok(MergeResult::Success(content)) => {
            write_merged_content(state, ch, &local_scope, doc_id, &content);
        }
        Ok(MergeResult::Conflict { local, remote, .. }) => {
            send_merge_conflict(state, ch, &local_scope, doc_id, local, remote);
        }
        Err(e) => errors::request_failed(ch, format!("Merge failed: {}", e)),
    }
}

fn write_merged_content(
    state: &Arc<AppState>,
    ch: &DualChannel,
    scope: &ResolvedRepo,
    doc_id: DocId,
    content: &str,
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
            return errors::request_failed(
                ch,
                format!("Failed to load local merge state: {}", err),
            );
        }
    };
    let patch = match reconcile::compute_reconcile_patch(&entries, content) {
        Ok(patch) => patch,
        Err(err) => {
            return errors::request_failed(ch, format!("Failed to diff merged content: {}", err));
        }
    };
    if let Err(err) = reconcile::append_patch_in_local_repo(
        &state.repo,
        &scope.repo_name,
        doc_id,
        "merge",
        &patch,
    ) {
        errors::storage_persist_failed(ch, format!("Failed to append merged content: {}", err));
        return;
    }
    if let Err(err) = state
        .sync_manager
        .persist_doc_in_local_repo(&scope.repo_name, doc_id)
    {
        errors::storage_persist_failed(ch, format!("Failed to persist merged content: {}", err));
        return;
    }
    tracing::info!("Merge Success for doc {}", doc_id);
    ch.broadcast(ServerMessage::MergeComplete {
        repo_id: Some(scope.repo_id),
        branch: scope.branch.clone(),
        merged_count: 1,
    });
}

fn send_merge_conflict(
    state: &Arc<AppState>,
    ch: &DualChannel,
    scope: &ResolvedRepo,
    doc_id: DocId,
    local: String,
    remote: String,
) {
    let Some(path) = resolve_doc_path(state, ch, &scope.repo_name, doc_id) else {
        return;
    };
    tracing::warn!("Merge Conflict detected for doc {}", doc_id);
    ch.unicast(ServerMessage::DocDiff {
        repo_id: Some(scope.repo_id),
        branch: scope.branch.clone(),
        path,
        old_content: local,
        new_content: remote,
    });
    errors::storage_conflict(ch, "Merge Conflict detected. Showing Diff View.");
}
