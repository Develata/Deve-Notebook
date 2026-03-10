// apps/cli/src/server/handlers/docs/delete.rs
//! # 删除文档处理器

use super::notify_fs_refresh;
use crate::server::AppState;
use crate::server::channel::DualChannel;
use crate::server::handlers::listing::handle_list_docs;
use crate::server::repo_scope::{
    local_repo_path, resolve_session_repo, run_on_resolved_local_repo,
};
use crate::server::session::WsSession;
use deve_core::ledger::node_meta;
use deve_core::protocol::ServerMessage;
use std::sync::Arc;

/// 处理删除文档请求
///
/// **流程**:
/// 1. 判断目标是文件还是目录
/// 2. 执行文件系统删除
/// 3. 从 Ledger 中移除记录
/// 4. 更新 TreeManager 并广播 TreeDelta
pub async fn handle_delete_doc(
    state: &Arc<AppState>,
    ch: &DualChannel,
    session: &mut WsSession,
    path: String,
) {
    // 只读模式检查: 静默忽略删除请求
    // TODO: Frontend will hide delete buttons when readonly
    if session.is_readonly() {
        tracing::debug!("Delete ignored: session is readonly (remote branch)");
        return;
    }
    let scope = match resolve_session_repo(state, session) {
        Ok(scope) => scope,
        Err(err) => {
            ch.send_error(err.to_string());
            return;
        }
    };

    tracing::info!("handle_delete_doc: path={}", path);
    let target = match local_repo_path(state, &scope, &path) {
        Ok(path) => path,
        Err(err) => {
            ch.send_error(err.to_string());
            return;
        }
    };
    let is_dir = target.is_dir();

    // 1. 获取 NodeId (用于 TreeDelta)
    let node_id = run_on_resolved_local_repo(state, &scope, |db| node_meta::get_node_id(db, &path))
        .ok()
        .flatten();

    // 3. 更新 Ledger
    if is_dir {
        if target.exists()
            && let Err(e) = std::fs::remove_dir_all(&target)
        {
            tracing::error!("删除目录失败 {}: {:?}", path, e);
            ch.send_error(format!("Failed to delete directory: {}", e));
            return;
        }
        match state
            .repo
            .delete_folder_in_local_repo(&scope.repo_name, &path)
        {
            Ok(count) => tracing::info!("已从 Ledger 删除 {} 个文档 (文件夹: {})", count, path),
            Err(e) => tracing::error!("Ledger 文件夹删除失败: {:?}", e),
        }
    } else {
        let doc_id = state
            .repo
            .get_docid_in_local_repo(&scope.repo_name, &path)
            .ok()
            .flatten();
        if let Err(e) = state.repo.apply_file_delete_structure_in_local_repo(
            &scope.repo_name,
            &path,
            doc_id,
            "local_delete",
        ) {
            tracing::error!("文件删除结构事实失败: {:?}", e);
            ch.send_error(format!("Failed to delete file: {}", e));
            return;
        }
        if target.exists()
            && let Err(e) = std::fs::remove_file(&target)
        {
            tracing::error!("删除文件失败 {}: {:?}", path, e);
            ch.send_error(format!("Failed to delete file: {}", e));
            return;
        }
    }

    // 4. 更新 TreeManager 并广播 Delta
    if let Some(node_id) = node_id {
        let delta = state
            .tree_manager
            .with_tree_mut(scope.repo_id, |tm| tm.remove(node_id));
        ch.unicast(ServerMessage::TreeUpdate(delta));
    }

    // 5. 刷新文档列表
    handle_list_docs(state, ch, session).await;
    notify_fs_refresh(ch, scope.repo_id, &path, "deleted");
}
