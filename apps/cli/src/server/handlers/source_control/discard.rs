//! # 放弃变更处理器 (Discard Handler)
//!
//! 从 staging.rs 拆分而来。恢复文件到已提交状态。
//!
//! **逻辑**: 获取已提交快照 → 计算与当前内容的差异 → 生成反向 Op → 应用到 Ledger

use crate::server::AppState;
use crate::server::channel::DualChannel;
use crate::server::repo_scope::run_on_resolved_local_repo;
use crate::server::session::WsSession;
use deve_core::protocol::ServerErrorCode;
use deve_core::protocol::ServerMessage;
use std::sync::Arc;

/// 放弃文件变更 (恢复到已提交状态)
pub async fn handle_discard_file(
    state: &Arc<AppState>,
    ch: &DualChannel,
    session: &WsSession,
    path: String,
) {
    let scope = match super::repo_scope::resolve_current_local_repo(state, session) {
        Ok(scope) => scope,
        Err(e) => return super::errors::send_ws(ch, e),
    };
    let (doc_id, committed_content, current_content) =
        match run_on_resolved_local_repo(state, &scope, |db| {
            let doc_id = deve_core::ledger::metadata::get_docid(db, &path)?
                .ok_or_else(|| anyhow::anyhow!("Document not found: {}", path))?;
            let committed_content =
                deve_core::source_control::changes::get_committed_content(db, doc_id)?
                    .unwrap_or_default();
            let ops = deve_core::ledger::ops::get_ops_from_db(db, doc_id)?;
            let entries: Vec<_> = ops.iter().map(|(_, e)| e.clone()).collect();
            Ok((
                doc_id,
                committed_content,
                deve_core::state::reconstruct_content(&entries),
            ))
        }) {
            Ok(payload) => payload,
            Err(e) => {
                return super::errors::send_ws(
                    ch,
                    super::errors::map_repo_error(super::errors::ScOp::DiffDoc(path.clone()), e),
                );
            }
        };

    if current_content == committed_content {
        tracing::info!("Discard: {} - 已与提交状态一致", path);
        ch.unicast(ServerMessage::DiscardAck { path: path.clone() });
        super::changes::handle_get_changes(state, ch, session).await;
        return;
    }

    // 计算差异并生成反向操作
    let ops = deve_core::state::compute_diff(&current_content, &committed_content);

    if let Err(e) = apply_reverse_ops(state, &scope.repo_name, doc_id, ops) {
        tracing::error!("Failed to discard {}: {:?}", path, e);
        return super::errors::send_ws(
            ch,
            super::errors::map_repo_error(super::errors::ScOp::DiscardPending(path.clone()), e),
        );
    }

    // 统一持久化到 Vault
    if let Err(e) = state
        .sync_manager
        .persist_doc_in_local_repo(&scope.repo_name, doc_id)
    {
        tracing::error!("持久化放弃内容失败: {:?}", e);
        return super::errors::send_ws_code(
            ch,
            ServerErrorCode::StoragePersistFailed,
            e.to_string(),
        );
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
    repo_name: &str,
    doc_id: deve_core::models::DocId,
    ops: Vec<deve_core::models::Op>,
) -> anyhow::Result<()> {
    let peer_id = state.identity_key.peer_id();
    for op in ops {
        let entry_peer_id = peer_id.clone();
        state.sync_manager.apply_local_op_in_local_repo(
            repo_name,
            doc_id,
            peer_id.clone(),
            move |seq| deve_core::models::LedgerEntry {
                doc_id,
                peer_id: entry_peer_id.clone(),
                seq,
                op: op.clone(),
                timestamp: chrono::Utc::now().timestamp_millis(),
            },
            false,
        )?;
    }
    Ok(())
}
