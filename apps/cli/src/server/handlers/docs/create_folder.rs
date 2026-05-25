//! plan_ref:
//!   - 10_rendering#document-authority-bridge
//!   - 04_repository#tree-projection-contract
//!   - 03_storage#watcher-contract
//!
//! 目录创建逻辑。

use super::checked_existing_is_dir;
use super::errors;
use super::node_helpers::broadcast_incremental_tree_deltas;
use super::notify_fs_refresh;
use crate::server::AppState;
use crate::server::channel::DualChannel;
use crate::server::repo_scope::ResolvedRepo;
use crate::server::session::WsSession;
use std::sync::Arc;

pub async fn handle_folder_create(
    state: &Arc<AppState>,
    ch: &DualChannel,
    session: &mut WsSession,
    scope: &ResolvedRepo,
    path: &std::path::Path,
    filename: &str,
) {
    let scope_nonce = session.is_browser_session().then(|| session.scope_nonce());
    let folder_path = filename.trim_end_matches('/');
    match checked_existing_is_dir(path, "folder create target") {
        Ok(Some(false)) => {
            tracing::error!("目标路径不是目录: {:?}", path);
            errors::storage_conflict_scoped(ch, "Target path is not a directory", scope_nonce);
            return;
        }
        Ok(Some(true)) | Ok(None) => {}
        Err(e) => {
            tracing::error!("检查目录创建目标失败: {:?}", e);
            errors::classified_failure_scoped(
                ch,
                format!("Failed to check folder target: {}", e),
                scope_nonce,
            );
            return;
        }
    }
    let ops = match state.repo.apply_dir_create_structure_in_local_repo(
        &scope.repo_name,
        folder_path,
        "local_create",
    ) {
        Ok((_node_id, ops)) => ops,
        Err(e) => {
            tracing::error!("目录结构事实追加失败: {:?}", e);
            errors::storage_persist_failed_scoped(
                ch,
                format!("Failed to create folder: {}", e),
                scope_nonce,
            );
            return;
        }
    };
    if let Err(e) = state
        .sync_manager
        .rebuild_projection_local_repo(&scope.repo_name)
    {
        tracing::error!("目录创建后重建投影失败: {:?}", e);
        errors::storage_persist_failed_scoped(
            ch,
            format!("Failed to rebuild created folder projection: {}", e),
            scope_nonce,
        );
        return;
    }
    if let Err(e) = broadcast_incremental_tree_deltas(state, ch, session, scope, &ops) {
        tracing::error!("目录创建后刷新视图失败: {:?}", e);
        errors::projection_refresh_failed_scoped(
            ch,
            format!("Failed to refresh created folder view: {}", e),
            scope_nonce,
        );
        return;
    }
    notify_fs_refresh(ch, scope.repo_id, folder_path, "dir-added");
}
