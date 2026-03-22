//! 文件创建逻辑。

use super::checked_exists;
use super::errors;
use super::file_register::create_file_from_content;
use super::node_helpers::broadcast_incremental_tree_deltas;
use super::notify_fs_refresh;
use crate::server::AppState;
use crate::server::channel::DualChannel;
use crate::server::repo_scope::ResolvedRepo;
use crate::server::session::WsSession;
use std::sync::Arc;

pub async fn handle_file_create(
    state: &Arc<AppState>,
    ch: &DualChannel,
    session: &mut WsSession,
    scope: &ResolvedRepo,
    path: &std::path::Path,
    filename: &str,
) {
    let scope_nonce = session.is_browser_session().then(|| session.scope_nonce());
    match checked_exists(path, "create target") {
        Ok(true) => {
            tracing::error!("目标路径已存在，拒绝从磁盘回填创建: {:?}", path);
            errors::storage_conflict_scoped(
                ch,
                format!("Target already exists: {}", filename),
                scope_nonce,
            );
            return;
        }
        Ok(false) => {}
        Err(e) => {
            tracing::error!("检查创建目标失败: {:?}", e);
            errors::classified_failure_scoped(
                ch,
                format!("Failed to check create target: {}", e),
                scope_nonce,
            );
            return;
        }
    }
    match state
        .repo
        .get_tracked_docid_in_local_repo(&scope.repo_name, filename)
    {
        Ok(Some(_)) => {
            errors::storage_conflict_scoped(
                ch,
                format!("Target already tracked: {}", filename),
                scope_nonce,
            );
            return;
        }
        Ok(None) => {}
        Err(e) => {
            tracing::error!("检查文件跟踪状态失败: {:?}", e);
            errors::classified_failure_scoped(
                ch,
                format!("Failed to check create target: {}", e),
                scope_nonce,
            );
            return;
        }
    }

    let ops = match create_file_from_content(state, scope, filename, "", "local_create") {
        Ok((_doc_id, ops)) => ops,
        Err(e) => {
            tracing::error!("文件创建失败: {:?}", e);
            errors::storage_persist_failed_scoped(
                ch,
                format!("Failed to create file: {}", e),
                scope_nonce,
            );
            return;
        }
    };

    if let Err(e) = broadcast_incremental_tree_deltas(state, ch, session, scope, &ops) {
        tracing::error!("文件创建后刷新视图失败: {:?}", e);
        errors::projection_refresh_failed_scoped(
            ch,
            format!("Failed to refresh created file view: {}", e),
            scope_nonce,
        );
        return;
    }

    notify_fs_refresh(ch, scope.repo_id, filename, "added");
}
