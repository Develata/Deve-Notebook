use crate::server::repo_scope::resolve_session_repo;
use crate::server::{AppState, channel::DualChannel, session::WsSession};
use deve_core::ledger::merge::MergeResult;
use deve_core::models::{DocId, PeerId};
use deve_core::protocol::ServerMessage;
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
            write_merged_content(state, ch, &scope.repo_name, doc_id, &content);
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
    repo_name: &str,
    doc_id: DocId,
    content: &str,
) {
    let Some(path) = resolve_doc_path(state, ch, repo_name, doc_id) else {
        return;
    };
    let abs_path = state.vault_path.join(&path);
    if let Err(e) = std::fs::write(&abs_path, content) {
        ch.send_error(format!("Failed to write merged content: {}", e));
        return;
    }
    tracing::info!("Merge Success for doc {} ({})", doc_id, path);
    ch.broadcast(ServerMessage::MergeComplete { merged_count: 1 });
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
        .get_path_by_docid_in_local_repo(repo_name, doc_id)
    {
        Ok(Some(path)) => Some(path),
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
