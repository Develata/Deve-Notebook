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

    // 2. 构造并追加操作 (Atomic Generation & Persist)
    // 使用 sync_manager.apply_local_op 自动处理序号生成和持久化
    let op_clone = op.clone();

    // 我们需要克隆 peer_id 用于构建 closure
    let peer_id_clone = local_peer_id.clone();

    match state.sync_manager.apply_local_op(
        doc_id,
        local_peer_id.clone(),
        move |seq| LedgerEntry {
            doc_id,
            op: op_clone.clone(),
            timestamp: chrono::Utc::now().timestamp_millis(),
            peer_id: peer_id_clone.clone(),
            seq,
        },
        true, // 自动写入 Vault
    ) {
        Ok((_global_seq, local_seq)) => {
            // 3. 广播新操作给所有连接的客户端
            // BUG FIX: 必须广播 Local Seq (CrDT Version)，而不是 Global Seq
            ch.broadcast(ServerMessage::NewOp {
                doc_id,
                op,
                seq: local_seq,
                client_id,
            });

            // 4. 发送 Ack
            ch.unicast(ServerMessage::Ack {
                doc_id,
                seq: local_seq,
            });
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
    active_repo: Option<&String>,
) {
    tracing::info!(
        "OpenDoc Request for DocID: {}, Branch: {:?}, Repo: {:?}",
        doc_id,
        active_branch,
        active_repo
    );

    let (final_content, version) = match active_branch {
        // 本地分支: 从本地 Ledger 读取 (Main or Extra)
        None => {
            let repo_name = active_repo
                .map(|s| s.as_str())
                .unwrap_or(state.repo.local_repo_name());

            // Reconcile logic: 暂时只对主库启用自动重整，避免多库冲突
            // TODO: Enhance SyncManager to support multi-repo reconciliation
            if repo_name == state.repo.local_repo_name() {
                if let Err(e) = state.sync_manager.reconcile_doc(doc_id) {
                    tracing::error!("SyncManager reconcile failed: {:?}", e);
                }
            }

            let res: anyhow::Result<Vec<(u64, LedgerEntry)>> =
                state.repo.run_on_local_repo(repo_name, |db| {
                    deve_core::ledger::ops::get_ops_from_db(db, doc_id)
                });

            match res {
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
                    tracing::error!("Failed to read ops from repo {}: {:?}", repo_name, e);
                    (String::new(), 0)
                }
            }
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
