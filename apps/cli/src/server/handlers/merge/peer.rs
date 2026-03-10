use crate::server::repo_scope::{ResolvedRepo, resolve_session_repo};
use crate::server::{AppState, channel::DualChannel, session::WsSession};
use deve_core::ledger::merge::MergeResult;
use deve_core::models::{DocId, PeerId};
use deve_core::protocol::ServerMessage;
use deve_core::sync::reconcile;
use std::sync::Arc;

/// 处理远端影子分支并入当前本地仓库。
///
/// Invariants:
/// - 合并目标必须是当前会话解析出的本地 repo。
/// - 远端影子分支内容绝不能写回到其他 repo 的 metadata/path 映射。
pub(super) async fn handle_merge_peer(
    state: &Arc<AppState>,
    ch: &DualChannel,
    session: &WsSession,
    peer_id: String,
    doc_id: DocId,
) {
    let scope = match resolve_session_repo(state, session) {
        Ok(scope) if scope.branch.is_none() => scope,
        Ok(scope) => {
            ch.send_error(format!(
                "Merge requested on remote branch context: {}",
                scope.repo_name
            ));
            return;
        }
        Err(e) => {
            ch.send_error(e.to_string());
            return;
        }
    };
    let peer_id = PeerId::new(peer_id);
    match state
        .repo
        .merge_peer_in_local_repo(&scope.repo_name, &peer_id, &scope.repo_id, doc_id)
    {
        Ok(MergeResult::Success(content)) => {
            write_merged_content(state, ch, &scope, doc_id, &content);
        }
        Ok(MergeResult::Conflict { local, remote, .. }) => {
            send_merge_conflict(state, ch, &scope.repo_name, doc_id, local, remote);
        }
        Err(e) => ch.send_error(format!("Merge failed: {}", e)),
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
        Err(err) => return ch.send_error(format!("Failed to load local merge state: {}", err)),
    };
    let patch = match reconcile::compute_reconcile_patch(&entries, content) {
        Ok(patch) => patch,
        Err(err) => return ch.send_error(format!("Failed to diff merged content: {}", err)),
    };
    if let Err(err) = reconcile::append_patch_in_local_repo(
        &state.repo,
        &scope.repo_name,
        doc_id,
        "merge",
        &patch,
    ) {
        ch.send_error(format!("Failed to append merged content: {}", err));
        return;
    }
    if let Err(err) = state
        .sync_manager
        .persist_doc_in_local_repo(&scope.repo_name, doc_id)
    {
        ch.send_error(format!("Failed to persist merged content: {}", err));
        return;
    }
    tracing::info!("Merge Success for doc {}", doc_id);
    ch.broadcast(ServerMessage::MergeComplete {
        repo_id: Some(scope.repo_id),
        merged_count: 1,
    });
}

fn send_merge_conflict(
    state: &Arc<AppState>,
    ch: &DualChannel,
    repo_name: &str,
    doc_id: DocId,
    local: String,
    remote: String,
) {
    let Some(path) = resolve_doc_path(state, ch, repo_name, doc_id) else {
        return;
    };
    tracing::warn!("Merge Conflict detected for doc {}", doc_id);
    ch.unicast(ServerMessage::DocDiff {
        path,
        old_content: local,
        new_content: remote,
    });
    ch.send_error("Merge Conflict detected. Showing Diff View.".to_string());
}

fn resolve_doc_path(
    state: &Arc<AppState>,
    ch: &DualChannel,
    repo_name: &str,
    doc_id: DocId,
) -> Option<String> {
    match state
        .repo
        .get_file_meta_for_doc_in_local_repo(repo_name, doc_id)
    {
        Ok(Some(meta)) => Some(meta.path),
        Ok(None) => {
            ch.send_error("Doc path not found for merged document".to_string());
            None
        }
        Err(e) => {
            ch.send_error(format!("Failed to resolve merged doc path: {}", e));
            None
        }
    }
}
