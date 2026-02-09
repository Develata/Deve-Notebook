use crate::server::AppState;
use crate::server::channel::DualChannel;
use crate::server::session::WsSession;
use deve_core::protocol::ServerMessage;
use std::collections::HashSet;
use std::sync::Arc;

/// 暂存指定文件
pub async fn handle_stage_file(state: &Arc<AppState>, ch: &DualChannel, path: String) {
    let path = deve_core::utils::path::to_forward_slash(&path);
    match state.repo.stage_file(&path) {
        Ok(()) => {
            tracing::info!("Staged file: {}", path);
            ch.unicast(ServerMessage::StageAck { path });
        }
        Err(e) => {
            tracing::error!("Failed to stage file: {:?}", e);
            ch.send_error(e.to_string());
        }
    }
}

/// 取消暂存指定文件
pub async fn handle_unstage_file(state: &Arc<AppState>, ch: &DualChannel, path: String) {
    let path = deve_core::utils::path::to_forward_slash(&path);
    match state.repo.unstage_file(&path) {
        Ok(()) => {
            tracing::info!("Unstaged file: {}", path);
            ch.unicast(ServerMessage::UnstageAck { path });
        }
        Err(e) => {
            tracing::error!("Failed to unstage file: {:?}", e);
            ch.send_error(e.to_string());
        }
    }
}

/// 批量暂存文件
pub async fn handle_stage_files(
    state: &Arc<AppState>,
    ch: &DualChannel,
    session: &WsSession,
    paths: Vec<String>,
) {
    for path in normalized_unique_paths(paths) {
        if let Err(e) = state.repo.stage_file(&path) {
            tracing::error!("Failed to stage file {}: {:?}", path, e);
            ch.send_error(e.to_string());
            return;
        }
    }
    super::changes::handle_get_changes(state, ch, session).await;
}

/// 批量取消暂存文件
pub async fn handle_unstage_files(
    state: &Arc<AppState>,
    ch: &DualChannel,
    session: &WsSession,
    paths: Vec<String>,
) {
    for path in normalized_unique_paths(paths) {
        if let Err(e) = state.repo.unstage_file(&path) {
            tracing::error!("Failed to unstage file {}: {:?}", path, e);
            ch.send_error(e.to_string());
            return;
        }
    }
    super::changes::handle_get_changes(state, ch, session).await;
}

fn normalized_unique_paths(paths: Vec<String>) -> Vec<String> {
    let mut seen = HashSet::new();
    paths
        .into_iter()
        .map(|p| deve_core::utils::path::to_forward_slash(&p))
        .filter(|p| !p.is_empty())
        .filter(|p| seen.insert(p.clone()))
        .collect()
}

/// 放弃文件变更 (恢复到已提交状态)
pub async fn handle_discard_file(
    state: &Arc<AppState>,
    ch: &DualChannel,
    session: &WsSession,
    path: String,
) {
    let doc_id = match state.repo.get_docid(&path) {
        Ok(Some(id)) => id,
        Ok(None) => {
            ch.send_error(format!("Document not found: {}", path));
            return;
        }
        Err(e) => {
            ch.send_error(e.to_string());
            return;
        }
    };

    // 获取已提交的快照内容
    let committed_content = state
        .repo
        .get_committed_content(doc_id)
        .ok()
        .flatten()
        .unwrap_or_default();

    // 实际恢复逻辑:
    // 1. 获取当前内容
    // 2. 计算差异 (current -> committed)
    // 3. 应用差异操作
    let current_content = state
        .repo
        .get_local_ops(doc_id)
        .ok()
        .map(|ops| {
            let entries: Vec<_> = ops.iter().map(|(_, e)| e.clone()).collect();
            deve_core::state::reconstruct_content(&entries)
        })
        .unwrap_or_default();

    if current_content == committed_content {
        tracing::info!("Discard file: {} - already matches committed state", path);
        ch.unicast(ServerMessage::DiscardAck { path: path.clone() });
        super::changes::handle_get_changes(state, ch, session).await;
        return;
    }

    // 计算差异并生成反向操作
    let ops = deve_core::state::compute_diff(&current_content, &committed_content);

    // 应用操作到 Ledger
    for op in ops {
        let peer_id = deve_core::models::PeerId::new("local");
        // 使用 SyncManager 应用 Op，但不每次都持久化 (为了性能)
        if let Err(e) = state.sync_manager.apply_local_op(
            doc_id,
            peer_id.clone(),
            move |seq| deve_core::models::LedgerEntry {
                doc_id,
                peer_id: deve_core::models::PeerId::new("local"),
                seq,
                op: op.clone(),
                timestamp: chrono::Utc::now().timestamp_millis(),
            },
            false, // 稍后一次性持久化
        ) {
            tracing::error!("Failed to apply discard op: {:?}", e);
            ch.send_error(format!("Failed to discard: {}", e));
            return;
        }
    }

    // 统一持久化到 Vault
    if let Err(e) = state.sync_manager.persist_doc(doc_id) {
        tracing::error!("Failed to persist discarded content: {:?}", e);
        ch.send_error(format!("Failed to persist discard: {}", e));
        return;
    }

    tracing::info!(
        "Discard file: {} (restored to {} bytes, was {} bytes)",
        path,
        committed_content.len(),
        current_content.len()
    );

    ch.unicast(ServerMessage::DiscardAck { path: path.clone() });
    // 刷新 Changes 列表
    super::changes::handle_get_changes(state, ch, session).await;
}
