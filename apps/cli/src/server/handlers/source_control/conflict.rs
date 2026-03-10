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
use deve_core::protocol::{ScPathTarget, ServerMessage};
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
    target: ScPathTarget,
    resolution: ConflictResolution,
) {
    let scope = match super::repo_scope::resolve_current_local_repo(state, session) {
        Ok(scope) => scope,
        Err(e) => return super::errors::send_ws(ch, e),
    };
    let selector = super::service::selector_from_scope(&scope);
    let pending = match super::service::list_pending(state.repo.as_ref(), &selector) {
        Ok(entries) => entries,
        Err(e) => return super::errors::send_ws(ch, e),
    };
    let normalized = super::service::resolve_path(&pending, &target);
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
            super::errors::send_ws(ch, e);
        }
    }
}

fn resolve_keep_fs(
    state: &Arc<AppState>,
    scope: &crate::server::repo_scope::ResolvedRepo,
    path: &str,
) -> Result<(), deve_core::protocol::ServerError> {
    run_on_resolved_local_repo(state, scope, |db| {
        let entry = deve_core::source_control::pending_fs::get(db, path)?
            .ok_or_else(|| anyhow::anyhow!("Pending change not found: {}", path))?;
        deve_core::source_control::pending_fs::remove(db, path)?;
        deve_core::ledger::source_control::stage_pending_entry(db, &entry)
    })
    .map_err(|e| {
        super::errors::map_repo_error(super::errors::ScOp::ResolveConflict(path.to_string()), e)
    })
}

fn resolve_keep_ledger(
    state: &Arc<AppState>,
    scope: &crate::server::repo_scope::ResolvedRepo,
    path: &str,
) -> Result<(), deve_core::protocol::ServerError> {
    let (projected, _) = state
        .repo
        .workdir_diff_inputs_in_local_repo(&scope.repo_name, path)
        .map_err(|e| {
            super::errors::map_repo_error(super::errors::ScOp::ResolveConflict(path.to_string()), e)
        })?;

    // 将 Ledger 内容写回磁盘
    let disk_path = local_repo_path(state, scope, path)
        .map_err(|e| super::errors::request_failed(e.to_string()))?;
    if let Some(parent) = disk_path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| super::errors::storage_persist_failed(e.to_string()))?;
    }
    std::fs::write(&disk_path, &projected)
        .map_err(|e| super::errors::storage_persist_failed(e.to_string()))?;

    // 移除 pending 条目
    run_on_resolved_local_repo(state, scope, |db| {
        deve_core::source_control::pending_fs::remove(db, path)
    })
    .map_err(|e| {
        super::errors::map_repo_error(super::errors::ScOp::ResolveConflict(path.to_string()), e)
    })
}
