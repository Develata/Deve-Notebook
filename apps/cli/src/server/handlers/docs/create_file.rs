//! 文件创建逻辑。

use super::errors;
use super::file_register::{broadcast_file_tree_update, create_file_from_content};
use super::notify_fs_refresh;
use crate::server::AppState;
use crate::server::channel::DualChannel;
use crate::server::handlers::listing::handle_list_docs;
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
    if path.exists() {
        tracing::error!("目标路径已存在，拒绝从磁盘回填创建: {:?}", path);
        errors::storage_conflict(ch, format!("Target already exists: {}", filename));
        return;
    }
    match state
        .repo
        .get_tracked_docid_in_local_repo(&scope.repo_name, filename)
    {
        Ok(Some(_)) => {
            errors::storage_conflict(ch, format!("Target already tracked: {}", filename));
            return;
        }
        Ok(None) => {}
        Err(e) => {
            tracing::error!("检查文件跟踪状态失败: {:?}", e);
            errors::request_failed(ch, format!("Failed to check create target: {}", e));
            return;
        }
    }

    let doc_id = match create_file_from_content(state, scope, filename, "", "local_create") {
        Ok(doc_id) => doc_id,
        Err(e) => {
            tracing::error!("文件创建失败: {:?}", e);
            errors::storage_persist_failed(ch, format!("Failed to create file: {}", e));
            return;
        }
    };

    broadcast_file_tree_update(state, ch, scope, doc_id, session.scope_nonce());
    handle_list_docs(state, ch, session, None, None).await;
    notify_fs_refresh(ch, scope.repo_id, filename, "added");
}
