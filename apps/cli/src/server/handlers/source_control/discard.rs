//! # 放弃变更处理器 (Discard Handler)
//!
//! 从 staging.rs 拆分而来。恢复文件到已提交状态。
//!
//! **逻辑**: 获取已提交快照 → 计算与当前内容的差异 → 生成反向 Op → 应用到 Ledger

use crate::server::AppState;
use crate::server::channel::DualChannel;
use crate::server::session::WsSession;
use deve_core::protocol::ServerMessage;
use std::sync::Arc;

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

    let committed_content = state
        .repo
        .get_committed_content(doc_id)
        .ok()
        .flatten()
        .unwrap_or_default();

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
        tracing::info!("Discard: {} - 已与提交状态一致", path);
        ch.unicast(ServerMessage::DiscardAck { path: path.clone() });
        super::changes::handle_get_changes(state, ch, session).await;
        return;
    }

    // 计算差异并生成反向操作
    let ops = deve_core::state::compute_diff(&current_content, &committed_content);

    if let Err(e) = apply_reverse_ops(state, doc_id, ops) {
        ch.send_error(format!("Failed to discard: {}", e));
        return;
    }

    // 统一持久化到 Vault
    if let Err(e) = state.sync_manager.persist_doc(doc_id) {
        tracing::error!("持久化放弃内容失败: {:?}", e);
        ch.send_error(format!("Failed to persist discard: {}", e));
        return;
    }

    tracing::info!(
        "Discard: {} (恢复到 {} 字节, 原 {} 字节)",
        path,
        committed_content.len(),
        current_content.len()
    );

    ch.unicast(ServerMessage::DiscardAck { path: path.clone() });
    super::changes::handle_get_changes(state, ch, session).await;
}

/// 将反向操作应用到 Ledger
///
/// **Invariant**: 每个 Op 按序应用，失败则立即中止。
fn apply_reverse_ops(
    state: &Arc<AppState>,
    doc_id: deve_core::models::DocId,
    ops: Vec<deve_core::models::Op>,
) -> anyhow::Result<()> {
    for op in ops {
        let peer_id = deve_core::models::PeerId::new("local");
        state.sync_manager.apply_local_op(
            doc_id,
            peer_id.clone(),
            move |seq| deve_core::models::LedgerEntry {
                doc_id,
                peer_id: deve_core::models::PeerId::new("local"),
                seq,
                op: op.clone(),
                timestamp: chrono::Utc::now().timestamp_millis(),
            },
            false,
        )?;
    }
    Ok(())
}
