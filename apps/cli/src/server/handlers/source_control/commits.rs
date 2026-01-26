use crate::server::AppState;
use crate::server::channel::DualChannel;
use deve_core::protocol::ServerMessage;
use std::sync::Arc;

/// 创建提交 (保存快照)
pub async fn handle_commit(state: &Arc<AppState>, ch: &DualChannel, message: String) {
    let get_content = |path: &str| -> Option<(deve_core::models::DocId, String)> {
        let normalized = deve_core::utils::path::to_forward_slash(path);
        let doc_id = state.repo.get_docid(&normalized).ok()??;
        let ops = state.repo.get_local_ops(doc_id).ok()?;
        let entries: Vec<_> = ops.iter().map(|(_, e)| e.clone()).collect();
        let content = deve_core::state::reconstruct_content(&entries);
        Some((doc_id, content))
    };

    match state
        .repo
        .create_commit_with_snapshots(&message, get_content)
    {
        Ok(info) => {
            tracing::info!("Created commit: {} - {}", info.id, info.message);
            // 广播提交成功 (其他标签页需要更新)
            ch.broadcast(ServerMessage::CommitAck {
                commit_id: info.id,
                timestamp: info.timestamp,
            });
        }
        Err(e) => {
            tracing::error!("Failed to create commit: {:?}", e);
            ch.send_error(e.to_string());
        }
    }
}

/// 获取提交历史
pub async fn handle_get_commit_history(state: &Arc<AppState>, ch: &DualChannel, limit: u32) {
    match state.repo.list_commits(limit) {
        Ok(commits) => {
            tracing::info!("Returning {} commits", commits.len());
            ch.unicast(ServerMessage::CommitHistory { commits });
        }
        Err(e) => {
            tracing::error!("Failed to get commit history: {:?}", e);
            ch.send_error(e.to_string());
        }
    }
}
