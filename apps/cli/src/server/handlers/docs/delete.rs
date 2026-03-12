// apps/cli/src/server/handlers/docs/delete.rs
//! # 删除文档处理器

use super::errors;
use super::node_target::resolve_node_target;
use super::notify_fs_refresh;
use crate::server::AppState;
use crate::server::channel::DualChannel;
use crate::server::handlers::listing::handle_list_docs;
use crate::server::repo_scope::{map_repo_scope_error, resolve_session_repo_and_sync};
use crate::server::session::WsSession;
use deve_core::models::NodeKind;
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
    if session.is_readonly() {
        tracing::debug!("Delete rejected: session is readonly (remote branch)");
        errors::remote_branch_readonly(ch);
        return;
    }
    let scope = match resolve_session_repo_and_sync(state, session) {
        Ok(scope) => scope,
        Err(err) => {
            ch.send_protocol_error(map_repo_scope_error(err));
            return;
        }
    };

    tracing::info!("handle_delete_doc: path={}", path);
    let target = match resolve_node_target(state, &scope, &path) {
        Ok(Some(target)) => target,
        Ok(None) => {
            errors::storage_not_found(ch, format!("Source not found: {}", path));
            return;
        }
        Err(err) => {
            errors::request_failed(ch, err.to_string());
            return;
        }
    };

    // 3. 更新 Ledger
    if target.kind == NodeKind::Dir {
        if let Err(e) = state.repo.apply_dir_delete_structure_in_local_repo(
            &scope.repo_name,
            &path,
            "local_delete",
        ) {
            tracing::error!("目录删除结构事实失败: {:?}", e);
            errors::storage_persist_failed(ch, format!("Failed to delete directory: {}", e));
            return;
        }
        if let Err(e) = state
            .sync_manager
            .rebuild_projection_local_repo(&scope.repo_name)
        {
            tracing::error!("目录删除后重建投影失败: {:?}", e);
            errors::storage_persist_failed(
                ch,
                format!("Failed to rebuild deleted directory projection: {}", e),
            );
            return;
        }
    } else {
        if let Err(e) = state.repo.apply_file_delete_structure_in_local_repo(
            &scope.repo_name,
            &path,
            target.doc_id,
            "local_delete",
        ) {
            tracing::error!("文件删除结构事实失败: {:?}", e);
            errors::storage_persist_failed(ch, format!("Failed to delete file: {}", e));
            return;
        }
        if let Err(e) = state
            .sync_manager
            .remove_projection_path_in_local_repo(&scope.repo_name, &path)
        {
            tracing::error!("删除文件投影失败 {}: {:?}", path, e);
            errors::storage_persist_failed(ch, format!("Failed to delete file: {}", e));
            return;
        }
    }

    // 4. 更新 TreeManager 并广播 Delta
    let delta = state
        .tree_manager
        .with_tree_mut(scope.repo_id, |tm| tm.remove(target.node_id));
    ch.unicast(ServerMessage::TreeUpdate {
        repo_id: Some(scope.repo_id),
        delta,
    });

    // 5. 刷新文档列表
    handle_list_docs(state, ch, session).await;
    notify_fs_refresh(ch, scope.repo_id, &path, "deleted");
}
