// apps/cli/src/server/handlers/document.rs
//! # 文档内容处理器
//!
//! 处理文档编辑、历史记录、打开等操作

use crate::server::AppState;
use crate::server::channel::DualChannel;
use crate::server::session::WsSession;
use deve_core::models::{LedgerEntry, Op};
use deve_core::protocol::ServerMessage;
use std::sync::Arc;
use std::time::Instant;

/// 处理编辑请求
///
/// **只读模式处理**:
/// 当 session 处于只读模式 (remotes 分支) 时，静默忽略编辑请求。
/// // TODO: Frontend will hide edit buttons when readonly
pub async fn handle_edit(
    state: &Arc<AppState>,
    ch: &DualChannel,
    session: &WsSession,
    doc_id: deve_core::models::DocId,
    op: deve_core::models::Op,
    client_id: u64,
) {
    // 只读模式检查: 静默忽略编辑请求
    if session.is_readonly() {
        tracing::debug!("Edit ignored: session is readonly (remote branch)");
        return;
    }

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
#[allow(dead_code)] // 历史回放功能预留
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
/// - `session`: WebSocket 会话，包含锁定的数据库
///
/// **逻辑**:
/// 使用 session 中锁定的 active_db 直接读取操作日志，
/// 支持本地和远程分支的统一读取。
pub async fn handle_open_doc(
    state: &Arc<AppState>,
    ch: &DualChannel,
    session: &WsSession,
    doc_id: deve_core::models::DocId,
) {
    tracing::info!(
        "OpenDoc Request for DocID: {}, Branch: {:?}, Repo: {:?}",
        doc_id,
        session.active_branch,
        session.active_repo
    );

    let start = Instant::now();

    // 优先使用 session 锁定的数据库
    let (snapshot_content, base_seq, delta_ops, version) = if let Some(handle) = session.get_active_db() {
        // 直接从锁定的数据库读取
        match build_snapshot_payload(&handle.db, doc_id, state.repo.snapshot_depth) {
            Ok(payload) => payload,
            Err(e) => {
                tracing::error!("Failed to build snapshot from active_db: {:?}", e);
                (String::new(), 0, Vec::new(), 0)
            }
        }
    } else {
        // 回退: 使用默认本地库
        tracing::warn!("No active_db in session, falling back to main local repo");
        let repo_name = state.repo.local_repo_name();

        // Reconcile logic for main repo
        if let Err(e) = state.sync_manager.reconcile_doc(doc_id) {
            tracing::error!("SyncManager reconcile failed: {:?}", e);
        }

        let res: anyhow::Result<(String, u64, Vec<(u64, Op)>, u64)> = state.repo.run_on_local_repo(
            repo_name,
            |db| build_snapshot_payload(db, doc_id, state.repo.snapshot_depth),
        );

        match res {
            Ok(payload) => payload,
            Err(e) => {
                tracing::error!("Failed to read snapshot from repo {}: {:?}", repo_name, e);
                (String::new(), 0, Vec::new(), 0)
            }
        }
    };

    tracing::info!(
        "OpenDoc Prepared: doc={}, base_seq={}, version={}, pending_ops={}, elapsed_ms={}",
        doc_id,
        base_seq,
        version,
        delta_ops.len(),
        start.elapsed().as_millis()
    );

    // 单播快照给请求者
    ch.unicast(ServerMessage::Snapshot {
        doc_id,
        content: snapshot_content,
        base_seq,
        version,
        delta_ops,
    });
}

fn build_snapshot_payload(
    db: &redb::Database,
    doc_id: deve_core::models::DocId,
    snapshot_depth: usize,
) -> anyhow::Result<(String, u64, Vec<(u64, Op)>, u64)> {
    let snapshot = deve_core::ledger::snapshot::load_latest_snapshot(db, doc_id)?;
    let has_snapshot = snapshot.is_some();
    let (base_seq, content) = snapshot.unwrap_or((0, String::new()));

    let delta_entries = deve_core::ledger::ops::get_ops_from_db_after(db, doc_id, base_seq)?;
    let mut version = base_seq;
    let mut delta_ops = Vec::new();
    for (seq, entry) in delta_entries {
        version = version.max(seq);
        delta_ops.push((seq, entry.op));
    }

    if !has_snapshot {
        let full_entries = deve_core::ledger::ops::get_ops_from_db(db, doc_id)?;
        if full_entries.is_empty() {
            return Ok((String::new(), 0, Vec::new(), 0));
        }

        let ops: Vec<LedgerEntry> = full_entries
            .iter()
            .map(|(_, entry)| entry.clone())
            .collect();
        let full_content = deve_core::state::reconstruct_content(&ops);
        let full_version = full_entries.last().map(|(seq, _)| *seq).unwrap_or(0);
        let _ = deve_core::ledger::snapshot::save_snapshot(
            db,
            doc_id,
            full_version,
            &full_content,
            snapshot_depth,
        );
        return Ok((full_content, full_version, Vec::new(), full_version));
    }

    if content.is_empty() && base_seq == 0 && !delta_ops.is_empty() {
        let full_entries = deve_core::ledger::ops::get_ops_from_db(db, doc_id)?;
        if full_entries.is_empty() {
            return Ok((String::new(), 0, Vec::new(), 0));
        }
        let ops: Vec<LedgerEntry> = full_entries
            .iter()
            .map(|(_, entry)| entry.clone())
            .collect();
        let full_content = deve_core::state::reconstruct_content(&ops);
        let full_version = full_entries.last().map(|(seq, _)| *seq).unwrap_or(0);
        let _ = deve_core::ledger::snapshot::save_snapshot(
            db,
            doc_id,
            full_version,
            &full_content,
            snapshot_depth,
        );
        return Ok((full_content, full_version, Vec::new(), full_version));
    }

    Ok((content, base_seq, delta_ops, version))
}
