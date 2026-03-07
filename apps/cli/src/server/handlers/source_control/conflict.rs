//! # 冲突解决处理器 (Conflict Resolution Handler)
//!
//! 处理 FS vs Ledger 冲突的用户决策。
//!
//! **KeepFs**: 将 FS 版本暂存 (stage_pending)，Ledger 内容被覆盖。
//! **KeepLedger**: 将 Ledger 内容写回磁盘，移除 pending 条目。

use crate::server::AppState;
use crate::server::channel::DualChannel;
use crate::server::repo_scope::{local_repo_path, run_on_resolved_local_repo};
use crate::server::session::WsSession;
use deve_core::protocol::ServerMessage;
use deve_core::source_control::ConflictResolution;
use std::sync::Arc;

/// 处理冲突解决请求
///
/// **Pre-condition**: path 对应的 pending_fs_ops 条目 has_conflict=true。
/// **Post-condition**: 冲突已解决，pending 条目已移除或更新。
pub async fn handle_resolve_conflict(
    state: &Arc<AppState>,
    ch: &DualChannel,
    session: &WsSession,
    path: String,
    resolution: ConflictResolution,
) {
    let normalized = deve_core::utils::path::to_forward_slash(&path);
    let scope = match super::repo_scope::resolve_current_local_repo(state, session) {
        Ok(scope) => scope,
        Err(e) => return ch.send_error(e.to_string()),
    };
    let result = match resolution {
        ConflictResolution::KeepFs => resolve_keep_fs(state, &scope, &normalized),
        ConflictResolution::KeepLedger => resolve_keep_ledger(state, &scope, &normalized),
    };

    match result {
        Ok(()) => {
            let label = match resolution {
                ConflictResolution::KeepFs => "KeepFs",
                ConflictResolution::KeepLedger => "KeepLedger",
            };
            tracing::info!("Conflict resolved: {} ({})", normalized, label);
            ch.unicast(ServerMessage::ConflictResolved {
                path: normalized,
                resolution: label.to_string(),
            });
            // 刷新变更列表
            super::changes::handle_get_changes(state, ch, session).await;
        }
        Err(e) => {
            tracing::error!("Failed to resolve conflict {}: {:?}", normalized, e);
            ch.send_error(e.to_string());
        }
    }
}

fn resolve_keep_fs(
    state: &Arc<AppState>,
    scope: &crate::server::repo_scope::ResolvedRepo,
    path: &str,
) -> anyhow::Result<()> {
    run_on_resolved_local_repo(state, scope, |db| {
        deve_core::source_control::pending_fs::get(db, path)?
            .ok_or_else(|| anyhow::anyhow!("Pending change not found: {}", path))?;
        let status = deve_core::source_control::pending_fs::get(db, path)?
            .map(|e| e.change_type)
            .unwrap_or(deve_core::source_control::ChangeStatus::Modified);
        deve_core::source_control::pending_fs::remove(db, path)?;
        deve_core::ledger::source_control::stage_file_with_status(db, path, status)
    })
}

fn resolve_keep_ledger(
    state: &Arc<AppState>,
    scope: &crate::server::repo_scope::ResolvedRepo,
    path: &str,
) -> anyhow::Result<()> {
    let committed = run_on_resolved_local_repo(state, scope, |db| {
        let doc_id = deve_core::ledger::metadata::get_docid(db, path)?
            .ok_or_else(|| anyhow::anyhow!("Document not found: {}", path))?;
        Ok(
            deve_core::source_control::changes::get_committed_content(db, doc_id)?
                .unwrap_or_default(),
        )
    })?;

    // 将 Ledger 内容写回磁盘
    let disk_path = local_repo_path(state, scope, path)?;
    if let Some(parent) = disk_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&disk_path, &committed)?;

    // 移除 pending 条目
    run_on_resolved_local_repo(state, scope, |db| {
        deve_core::source_control::pending_fs::remove(db, path)
    })
}
