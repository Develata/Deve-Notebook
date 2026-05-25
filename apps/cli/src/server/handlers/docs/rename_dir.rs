//! plan_ref:
//!   - 10_rendering#document-authority-bridge
//!   - 04_repository#tree-projection-contract
//!   - 03_storage#watcher-contract

use super::errors;
use super::node_helpers::broadcast_incremental_tree_deltas;
use super::notify_fs_refresh;
use crate::server::AppState;
use crate::server::channel::DualChannel;
use crate::server::repo_scope::ResolvedRepo;
use crate::server::session::WsSession;
use std::sync::Arc;

pub(super) async fn handle_dir_rename(
    state: &Arc<AppState>,
    ch: &DualChannel,
    session: &mut WsSession,
    scope: &ResolvedRepo,
    old_path: &str,
    dst_name: &str,
) {
    let scope_nonce = session.is_browser_session().then(|| session.scope_nonce());
    let ops = match state.repo.apply_dir_rename_structure_in_local_repo(
        &scope.repo_name,
        old_path,
        dst_name,
        "local_rename",
    ) {
        Ok(Some((_node_id, ops))) => ops,
        Ok(None) => {
            errors::storage_not_found_scoped(
                ch,
                format!("Source not tracked: {}", old_path),
                scope_nonce,
            );
            return;
        }
        Err(e) => {
            tracing::error!("目录重命名结构事实失败: {:?}", e);
            errors::storage_persist_failed_scoped(
                ch,
                format!("Failed to rename folder: {}", e),
                scope_nonce,
            );
            return;
        }
    };
    if let Err(e) = state
        .sync_manager
        .rebuild_projection_local_repo(&scope.repo_name)
    {
        tracing::error!("目录重命名后重建投影失败: {:?}", e);
        errors::storage_persist_failed_scoped(
            ch,
            format!("Failed to rebuild renamed directory projection: {}", e),
            scope_nonce,
        );
        return;
    }
    if let Err(e) = broadcast_incremental_tree_deltas(state, ch, session, scope, &ops) {
        tracing::error!("目录重命名后刷新视图失败: {:?}", e);
        errors::projection_refresh_failed_scoped(
            ch,
            format!("Failed to refresh renamed folder view: {}", e),
            scope_nonce,
        );
        return;
    }
    notify_fs_refresh(ch, scope.repo_id, dst_name, "renamed");
}
