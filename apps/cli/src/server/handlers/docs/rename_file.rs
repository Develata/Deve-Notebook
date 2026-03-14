use super::errors;
use super::node_helpers::broadcast_local_projection_refresh;
use super::notify_fs_refresh;
use crate::server::AppState;
use crate::server::channel::DualChannel;
use crate::server::repo_scope::ResolvedRepo;
use crate::server::session::WsSession;
use std::sync::Arc;

pub(super) async fn handle_file_rename(
    state: &Arc<AppState>,
    ch: &DualChannel,
    session: &mut WsSession,
    scope: &ResolvedRepo,
    src_path: &str,
    dst_path: &str,
) {
    let doc_id = match state
        .repo
        .get_tracked_docid_in_local_repo(&scope.repo_name, src_path)
    {
        Ok(Some(doc_id)) => doc_id,
        Ok(None) => {
            errors::storage_not_found(ch, format!("Document not tracked: {}", src_path));
            return;
        }
        Err(e) => {
            errors::classified_failure(ch, format!("Failed to resolve document: {}", e));
            return;
        }
    };
    if let Err(e) = state.repo.apply_file_structure_in_local_repo(
        &scope.repo_name,
        dst_path,
        Some(doc_id),
        "local_rename",
    ) {
        tracing::error!("重命名结构事实失败: {:?}", e);
        errors::storage_persist_failed(ch, format!("Failed to rename file: {}", e));
        return;
    }
    if let Err(e) = state
        .sync_manager
        .persist_doc_in_local_repo(&scope.repo_name, doc_id)
    {
        tracing::error!("重命名投影持久化失败: {:?}", e);
        errors::storage_persist_failed(ch, format!("Failed to materialize renamed file: {}", e));
        return;
    }
    if let Err(e) = state
        .sync_manager
        .remove_projection_path_in_local_repo(&scope.repo_name, src_path)
    {
        tracing::error!("旧路径投影清理失败 {}: {:?}", src_path, e);
        errors::storage_persist_failed(ch, format!("Failed to remove old file: {}", e));
        return;
    }
    if let Err(e) = broadcast_local_projection_refresh(state, ch, session, scope) {
        tracing::error!("文件重命名后刷新视图失败: {:?}", e);
        errors::projection_refresh_failed(
            ch,
            format!("Failed to refresh renamed file view: {}", e),
        );
        return;
    }
    notify_fs_refresh(ch, scope.repo_id, dst_path, "renamed");
}
