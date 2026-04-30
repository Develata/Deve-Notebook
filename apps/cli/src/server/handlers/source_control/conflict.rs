//! plan_ref:
//!   - 07_diff_logic#source-control-runtime
//!   - 04_storage#watcher-contract
//!
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
use deve_core::source_control::{ChangeEntry, ConflictResolution};
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
    let scope_nonce = session.is_browser_session().then(|| session.scope_nonce());
    let scope = match super::repo_scope::resolve_current_local_repo(state, session) {
        Ok(scope) => scope,
        Err(e) => return super::errors::send_ws_scoped(ch, e, scope_nonce),
    };
    let selector = super::service::selector_from_scope(&scope);
    let pending = match super::service::list_pending(state.repo.as_ref(), &selector) {
        Ok(entries) => entries,
        Err(e) => return super::errors::send_ws_scoped(ch, e, scope_nonce),
    };
    let resolved = match super::service::resolve_target(&pending, &target) {
        Ok(resolved) => resolved,
        Err(e) => return super::errors::send_ws_scoped(ch, e, scope_nonce),
    };
    let resolved_entry = match super::service::resolved_target_entry(&pending, &resolved) {
        Ok(entry) => entry,
        Err(e) => return super::errors::send_ws_scoped(ch, e, scope_nonce),
    };
    if !resolved_entry.has_conflict {
        return super::errors::send_ws_scoped(
            ch,
            deve_core::protocol::ServerError::with_detail(
                deve_core::protocol::ServerErrorCode::ScConflictTargetMissing,
                format!("Source control target is not a conflict: {}", resolved.path),
            ),
            scope_nonce,
        );
    }
    let normalized = resolved.path.clone();
    let result = match resolution {
        ConflictResolution::KeepFs => resolve_keep_fs(state, &selector, &pending, &resolved),
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
            super::errors::send_ws_scoped(ch, e, scope_nonce);
        }
    }
}

fn resolve_keep_fs(
    state: &Arc<AppState>,
    selector: &RepoSelector,
    pending: &[ChangeEntry],
    target: &ScPathTarget,
) -> Result<(), deve_core::protocol::ServerError> {
    let repo_name = selector.repo_name.as_deref().ok_or_else(|| {
        deve_core::protocol::ServerError::with_detail(
            deve_core::protocol::ServerErrorCode::ScRepoContextInvalid,
            "Missing local repo selector for conflict resolution",
        )
    })?;
    let related_targets = super::service::related_targets(pending, target)?;
    state
        .repo
        .stage_resolved_pending_targets_in_local_repo(repo_name, &related_targets)
        .map_err(|e| {
            super::errors::map_repo_error(super::errors::ScOp::StagePending(target.path.clone()), e)
        })
}

fn resolve_keep_ledger(
    state: &Arc<AppState>,
    selector: &RepoSelector,
    target: &ScPathTarget,
) -> Result<(), deve_core::protocol::ServerError> {
    super::local_discard::discard_via_sync_manager(state, selector, target).map(|_| ())
}
