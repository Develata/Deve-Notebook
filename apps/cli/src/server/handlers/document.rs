// apps/cli/src/server/handlers/document.rs
//! # 文档内容处理器
//!
//! 处理文档编辑、历史记录、打开等操作

use crate::server::AppState;
use crate::server::channel::DualChannel;
use deve_core::models::{LedgerEntry, PeerId};
use deve_core::protocol::ServerMessage;
use std::sync::Arc;

/// 处理编辑请求
pub async fn handle_edit(
    state: &Arc<AppState>,
    ch: &DualChannel,
    doc_id: deve_core::models::DocId,
    op: deve_core::models::Op,
    client_id: u64,
) {
    // 获取本地 Peer ID
    let local_peer_id = state.identity_key.peer_id();

    // 计算下一个序列号
    let mut next_seq = 1;
    if let Ok(ops) = state.repo.get_local_ops(doc_id) {
        let max_seq = ops
            .iter()
            .filter(|(_, e)| e.peer_id == local_peer_id)
            .map(|(_, e)| e.seq)
            .max()
            .unwrap_or(0);
        next_seq = max_seq + 1;
    }

    let entry = LedgerEntry {
        doc_id,
        op: op.clone(),
        timestamp: chrono::Utc::now().timestamp_millis(),
        peer_id: local_peer_id,
        seq: next_seq,
    };

    match state.repo.append_local_op(&entry) {
        Ok(seq) => {
            // 广播新操作给所有连接的客户端
            ch.broadcast(ServerMessage::NewOp {
                doc_id,
                op,
                seq,
                client_id,
            });

            // 持久化到磁盘
            if let Err(e) = state.sync_manager.persist_doc(doc_id) {
                tracing::error!("SyncManager failed to persist doc {}: {:?}", doc_id, e);
            }
        }
        Err(e) => {
            tracing::error!("Failed to persist op: {:?}", e);
            ch.send_error(format!("Failed to persist operation: {}", e));
        }
    }
}

/// 处理历史记录请求
pub async fn handle_request_history(
    state: &Arc<AppState>,
    ch: &DualChannel,
    doc_id: deve_core::models::DocId,
) {
    if let Ok(entries) = state.repo.get_local_ops(doc_id) {
        let ops: Vec<(u64, deve_core::models::Op)> = entries
            .into_iter()
            .map(|(seq, entry)| (seq, entry.op))
            .collect();

        // 单播历史记录给请求者
        ch.unicast(ServerMessage::History { doc_id, ops });
    }
}

/// 打开文档
///
/// **参数**:
/// - `active_branch`: 当前活动分支。None = 本地, Some = 影子库
pub async fn handle_open_doc(
    state: &Arc<AppState>,
    ch: &DualChannel,
    doc_id: deve_core::models::DocId,
    active_branch: Option<&PeerId>,
) {
    tracing::info!(
        "OpenDoc Request for DocID: {}, Branch: {:?}",
        doc_id,
        active_branch
    );

    let (final_content, version) = match active_branch {
        // 本地分支: 从本地 Ledger 读取
        None => {
            // 调和: 确保磁盘内容与 Ledger 一致
            if let Err(e) = state.sync_manager.reconcile_doc(doc_id) {
                tracing::error!("SyncManager reconcile failed: {:?}", e);
            }

            let entries_with_seq = state.repo.get_local_ops(doc_id).unwrap_or_default();
            let ops: Vec<LedgerEntry> = entries_with_seq
                .iter()
                .map(|(_, entry)| entry.clone())
                .collect();
            let content = deve_core::state::reconstruct_content(&ops);
            let ver = entries_with_seq.last().map(|(seq, _)| *seq).unwrap_or(0);
            (content, ver)
        }
        // 影子分支: 从 Shadow DB 读取
        Some(peer_id) => {
            match state
                .repo
                .get_shadow_ops(peer_id, &uuid::Uuid::nil(), doc_id)
            {
                Ok(entries_with_seq) => {
                    let ops: Vec<LedgerEntry> = entries_with_seq
                        .iter()
                        .map(|(_, entry)| entry.clone())
                        .collect();
                    let content = deve_core::state::reconstruct_content(&ops);
                    let ver = entries_with_seq.last().map(|(seq, _)| *seq).unwrap_or(0);
                    (content, ver)
                }
                Err(e) => {
                    tracing::error!("Failed to get shadow ops for {}: {:?}", peer_id, e);
                    (String::new(), 0)
                }
            }
        }
    };

    // 单播快照给请求者
    ch.unicast(ServerMessage::Snapshot {
        doc_id,
        content: final_content,
        version,
    });
}
