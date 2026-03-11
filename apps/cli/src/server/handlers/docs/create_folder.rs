//! 目录创建逻辑。

use super::errors;
use super::notify_fs_refresh;
use crate::server::AppState;
use crate::server::channel::DualChannel;
use crate::server::handlers::docs::node_helpers::broadcast_dir_chain;
use crate::server::handlers::listing::handle_list_docs;
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
    let folder_path = filename.trim_end_matches('/');
    if path.exists() && !path.is_dir() {
        tracing::error!("目标路径不是目录: {:?}", path);
        errors::storage_conflict(ch, "Target path is not a directory");
        return;
    }
    let node_id = match state.repo.apply_dir_create_structure_in_local_repo(
        &scope.repo_name,
        folder_path,
        "local_create",
    ) {
        Ok(node_id) => node_id,
        Err(e) => {
            tracing::error!("目录结构事实追加失败: {:?}", e);
            errors::storage_persist_failed(ch, format!("Failed to create folder: {}", e));
            return;
        }
    };
    if let Err(e) = state
        .sync_manager
        .rebuild_projection_local_repo(&scope.repo_name)
    {
        tracing::error!("目录创建后重建投影失败: {:?}", e);
        errors::storage_persist_failed(
            ch,
            format!("Failed to rebuild created folder projection: {}", e),
        );
        return;
    }
    if let Err(e) = broadcast_dir_chain(state, ch, scope.repo_id, &scope.repo_name, node_id) {
        tracing::error!("广播目录链失败: {:?}", e);
    }
    handle_list_docs(state, ch, session).await;
    notify_fs_refresh(ch, scope.repo_id, folder_path, "dir-added");
}
