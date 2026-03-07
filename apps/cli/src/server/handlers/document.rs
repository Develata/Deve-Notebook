// apps/cli/src/server/handlers/document.rs
//! # 文档内容处理器
//!
//! 处理文档编辑、历史记录、打开等操作

use crate::server::AppState;
use crate::server::channel::DualChannel;
use crate::server::repo_scope::resolve_session_repo;
use crate::server::session::WsSession;
use deve_core::models::{LedgerEntry, Op};
use deve_core::protocol::ServerMessage;
use std::sync::Arc;
use std::time::Instant;

/// 快照结果类型: (content, base_seq, delta_ops, version)
type SnapshotPayload = (String, u64, Vec<(u64, Op)>, u64);

/// 处理编辑请求。
pub async fn handle_edit(
    state: &Arc<AppState>,
    ch: &DualChannel,
    session: &WsSession,
    doc_id: deve_core::models::DocId,
    op: deve_core::models::Op,
    client_id: u64,
) {
    if session.is_readonly() {
        tracing::debug!("Edit ignored: session is readonly (remote branch)");
        return;
    }
    let scope = match resolve_session_repo(state, session) {
        Ok(scope) => scope,
        Err(e) => return ch.send_error(e.to_string()),
    };
    let local_peer_id = state.identity_key.peer_id();
    let op_clone = op.clone();
    let peer_id_clone = local_peer_id.clone();
    match state.sync_manager.apply_local_op_in_local_repo(
        &scope.repo_name,
        doc_id,
        local_peer_id,
        move |seq| LedgerEntry {
            doc_id,
            op: op_clone.clone(),
            timestamp: chrono::Utc::now().timestamp_millis(),
            peer_id: peer_id_clone.clone(),
            seq,
        },
        true,
    ) {
        Ok((_global_seq, local_seq)) => {
            ch.broadcast(ServerMessage::NewOp {
                doc_id,
                op,
                seq: local_seq,
                client_id,
            });
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

/// 打开文档。
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

    let (snapshot_content, base_seq, delta_ops, version) = if let Some(handle) =
        session.get_active_db()
    {
        match build_snapshot_payload(&handle.db, doc_id, state.repo.snapshot_depth) {
            Ok(payload) => payload,
            Err(e) => {
                tracing::error!("Failed to build snapshot from active_db: {:?}", e);
                (String::new(), 0, Vec::new(), 0)
            }
        }
    } else {
        let repo_name = resolve_session_repo(state, session)
            .map(|scope| scope.repo_name)
            .unwrap_or_else(|_| state.repo.local_repo_name().to_string());
        tracing::warn!(
            "No active_db in session, falling back to resolved local repo {}",
            repo_name
        );
        if let Err(e) = state
            .sync_manager
            .reconcile_doc_in_local_repo(&repo_name, doc_id)
        {
            tracing::error!("SyncManager reconcile failed: {:?}", e);
        }

        let res: anyhow::Result<SnapshotPayload> = state.repo.run_on_local_repo(&repo_name, |db| {
            build_snapshot_payload(db, doc_id, state.repo.snapshot_depth)
        });

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
) -> anyhow::Result<SnapshotPayload> {
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
