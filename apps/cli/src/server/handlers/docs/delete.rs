// apps/cli/src/server/handlers/docs/delete.rs
//! # 删除文档处理器

use super::errors;
use super::node_helpers::broadcast_local_projection_refresh;
use super::node_target::resolve_node_target;
use super::notify_fs_refresh;
use crate::server::AppState;
use crate::server::channel::DualChannel;
use crate::server::repo_scope::{map_repo_scope_error, resolve_session_repo_and_sync};
use crate::server::session::WsSession;
use deve_core::models::NodeKind;
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
    let scope_nonce = session.is_browser_session().then(|| session.scope_nonce());
    if session.is_readonly() {
        tracing::debug!("Delete rejected: session is readonly (remote branch)");
        errors::remote_branch_readonly_scoped(ch, scope_nonce);
        return;
    }
    let scope = match resolve_session_repo_and_sync(state, session) {
        Ok(scope) => scope,
        Err(err) => {
            ch.send_protocol_error_with_scope_nonce(map_repo_scope_error(err), scope_nonce);
            return;
        }
    };

    tracing::info!("handle_delete_doc: path={}", path);
    let target = match resolve_node_target(state, &scope, &path) {
        Ok(Some(target)) => target,
        Ok(None) => {
            errors::storage_not_found_scoped(
                ch,
                format!("Source not found: {}", path),
                scope_nonce,
            );
            return;
        }
        Err(err) => {
            errors::classified_failure_scoped(ch, err.to_string(), scope_nonce);
            return;
        }
    };

    // 3. 更新 Ledger
    if target.kind == NodeKind::Dir {
        if let Err(e) = state.repo.apply_dir_delete_structure_in_local_repo(
            &scope.repo_name,
            &target.repo_path,
            "local_delete",
        ) {
            tracing::error!("目录删除结构事实失败: {:?}", e);
            errors::storage_persist_failed_scoped(
                ch,
                format!("Failed to delete directory: {}", e),
                scope_nonce,
            );
            return;
        }
        if let Err(e) = state
            .sync_manager
            .rebuild_projection_local_repo(&scope.repo_name)
        {
            tracing::error!("目录删除后重建投影失败: {:?}", e);
            errors::storage_persist_failed_scoped(
                ch,
                format!("Failed to rebuild deleted directory projection: {}", e),
                scope_nonce,
            );
            return;
        }
    } else {
        if let Err(e) = state.repo.apply_file_delete_structure_in_local_repo(
            &scope.repo_name,
            &target.repo_path,
            target.doc_id,
            "local_delete",
        ) {
            tracing::error!("文件删除结构事实失败: {:?}", e);
            errors::storage_persist_failed_scoped(
                ch,
                format!("Failed to delete file: {}", e),
                scope_nonce,
            );
            return;
        }
        if let Err(e) = state
            .sync_manager
            .remove_projection_path_in_local_repo(&scope.repo_name, &target.repo_path)
        {
            tracing::error!("删除文件投影失败 {}: {:?}", target.repo_path, e);
            errors::storage_persist_failed_scoped(
                ch,
                format!("Failed to delete file: {}", e),
                scope_nonce,
            );
            return;
        }
    }

    if let Err(e) = broadcast_local_projection_refresh(state, ch, session, &scope) {
        tracing::error!("删除后刷新视图失败: {:?}", e);
        errors::projection_refresh_failed_scoped(
            ch,
            format!("Failed to refresh deleted node view: {}", e),
            scope_nonce,
        );
        return;
    }
    notify_fs_refresh(ch, scope.repo_id, &target.repo_path, "deleted");
}
