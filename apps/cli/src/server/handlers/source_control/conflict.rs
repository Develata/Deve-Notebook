//! # 冲突解决处理器 (Conflict Resolution Handler)
//!
//! 处理 FS vs Ledger 冲突的用户决策。
//!
//! **KeepFs**: 将 FS 版本暂存 (stage_pending)，Ledger 内容被覆盖。
//! **KeepLedger**: 将 Ledger 内容写回磁盘，移除 pending 条目。

use crate::server::AppState;
use crate::server::channel::DualChannel;
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
    let result = match resolution {
        ConflictResolution::KeepFs => resolve_keep_fs(state, &normalized),
        ConflictResolution::KeepLedger => resolve_keep_ledger(state, &normalized),
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

/// KeepFs: 将 FS 版本暂存 (Working Dir → Staging)
fn resolve_keep_fs(state: &Arc<AppState>, path: &str) -> anyhow::Result<()> {
    state.repo.stage_pending(path)
}

/// KeepLedger: 将 Ledger 已提交内容写回磁盘，移除 pending 条目
fn resolve_keep_ledger(state: &Arc<AppState>, path: &str) -> anyhow::Result<()> {
    let doc_id = state
        .repo
        .get_docid(path)?
        .ok_or_else(|| anyhow::anyhow!("Document not found: {}", path))?;

    let committed = state.repo.get_committed_content(doc_id)?.unwrap_or_default();

    // 将 Ledger 内容写回磁盘
    let disk_path = state.vault_path.join(path);
    if let Some(parent) = disk_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&disk_path, &committed)?;

    // 移除 pending 条目
    state.repo.discard_pending(path)
}