//! # 冲突解决处理器 (Conflict Resolution Handler)
//!
//! 处理 FS vs Ledger 冲突的用户决策。
//!
//! **KeepFs**: 将 FS 版本暂存 (stage_pending)，Ledger 内容被覆盖。
//! **KeepLedger**: 将 Ledger 内容写回磁盘，移除 pending 条目。

use crate::server::AppState;
use crate::server::channel::DualChannel;
use crate::server::session::WsSession;
use deve_core::ledger::traits::RepoSelector;
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
    session: &mut WsSession,
    target: ScPathTarget,
    resolution: ConflictResolution,
) {
    let scope_nonce = Some(session.scope_nonce());
    let scope = match super::repo_scope::resolve_current_local_repo(state, session) {
        Ok(scope) => scope,
        Err(e) => return super::errors::send_ws(ch, e),
    };
    let selector = super::service::selector_from_scope(&scope);
    let pending = match super::service::list_pending(state.repo.as_ref(), &selector) {
        Ok(entries) => entries,
        Err(e) => return super::errors::send_ws(ch, e),
    };
    let resolved = match super::service::resolve_target(&pending, &target) {
        Ok(resolved) => resolved,
        Err(e) => return super::errors::send_ws(ch, e),
    };
    let normalized = resolved.path.clone();
    let result = match resolution {
        ConflictResolution::KeepFs => resolve_keep_fs(state, &selector, &resolved),
        ConflictResolution::KeepLedger => resolve_keep_ledger(state, &selector, &resolved),
    };

    match result {
        Ok(()) => {
            let label = match resolution {
                ConflictResolution::KeepFs => "KeepFs",
                ConflictResolution::KeepLedger => "KeepLedger",
            };
            tracing::info!("Conflict resolved: {} ({})", normalized, label);
            ch.unicast(ServerMessage::ConflictResolved {
                repo_id: Some(scope.repo_id),
                branch: scope.branch.clone(),
                scope_nonce,
                path: normalized,
                resolution: label.to_string(),
            });
            // 刷新变更列表
            super::changes::handle_get_changes(state, ch, session, None).await;
        }
        Err(e) => {
            tracing::error!("Failed to resolve conflict {}: {:?}", normalized, e);
            super::errors::send_ws(ch, e);
        }
    }
}

fn resolve_keep_fs(
    state: &Arc<AppState>,
    selector: &RepoSelector,
    target: &ScPathTarget,
) -> Result<(), deve_core::protocol::ServerError> {
    super::service::stage_pending(state.repo.as_ref(), selector, target).map(|_| ())
}

fn resolve_keep_ledger(
    state: &Arc<AppState>,
    selector: &RepoSelector,
    target: &ScPathTarget,
) -> Result<(), deve_core::protocol::ServerError> {
    super::local_discard::discard_via_sync_manager(state, selector, target).map(|_| ())
}
