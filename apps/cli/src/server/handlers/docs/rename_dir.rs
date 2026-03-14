use super::errors;
use super::node_helpers::broadcast_local_projection_refresh;
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
    match state.repo.apply_dir_rename_structure_in_local_repo(
        &scope.repo_name,
        old_path,
        dst_name,
        "local_rename",
    ) {
        Ok(Some(_)) => {}
        Ok(None) => {
            errors::storage_not_found(ch, format!("Source not tracked: {}", old_path));
            return;
        }
        Err(e) => {
            tracing::error!("目录重命名结构事实失败: {:?}", e);
            errors::storage_persist_failed(ch, format!("Failed to rename folder: {}", e));
            return;
        }
    };
    if let Err(e) = state
        .sync_manager
        .rebuild_projection_local_repo(&scope.repo_name)
    {
        tracing::error!("目录重命名后重建投影失败: {:?}", e);
        errors::storage_persist_failed(
            ch,
            format!("Failed to rebuild renamed directory projection: {}", e),
        );
        return;
    }
    if let Err(e) = broadcast_local_projection_refresh(state, ch, session, scope) {
        tracing::error!("目录重命名后刷新视图失败: {:?}", e);
        errors::projection_refresh_failed(
            ch,
            format!("Failed to refresh renamed folder view: {}", e),
        );
        return;
    }
    notify_fs_refresh(ch, scope.repo_id, dst_name, "renamed");
}
