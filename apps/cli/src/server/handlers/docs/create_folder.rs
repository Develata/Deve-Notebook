//! 目录创建逻辑。

use super::errors;
use super::node_helpers::broadcast_local_projection_refresh;
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
    if path.exists() && !path.is_dir() {
        tracing::error!("目标路径不是目录: {:?}", path);
        errors::storage_conflict_scoped(ch, "Target path is not a directory", scope_nonce);
        return;
    }
    if let Err(e) = state.repo.apply_dir_create_structure_in_local_repo(
        &scope.repo_name,
        folder_path,
        "local_create",
    ) {
        tracing::error!("目录结构事实追加失败: {:?}", e);
        errors::storage_persist_failed_scoped(
            ch,
            format!("Failed to create folder: {}", e),
            scope_nonce,
        );
        return;
    }
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
    if let Err(e) = broadcast_local_projection_refresh(state, ch, session, scope) {
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
