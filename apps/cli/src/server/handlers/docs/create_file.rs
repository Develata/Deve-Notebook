//! 文件创建与已有文件注册逻辑。

use super::errors;
use super::file_register::{broadcast_file_tree_update, register_file_from_disk};
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
    let doc_id = match register_file_from_disk(state, scope, path, filename, "local_create") {
        Ok(doc_id) => doc_id,
        Err(e) => {
            tracing::error!("文件注册失败: {:?}", e);
            errors::storage_persist_failed(ch, format!("Failed to register file: {}", e));
            return;
        }
    };

    broadcast_file_tree_update(state, ch, scope, doc_id);
    handle_list_docs(state, ch, session).await;
    notify_fs_refresh(ch, scope.repo_id, filename, "added");
}
