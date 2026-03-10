use super::notify_fs_refresh;
use crate::server::AppState;
use crate::server::channel::DualChannel;
use crate::server::handlers::docs::node_helpers::broadcast_parent_dirs;
use crate::server::handlers::listing::handle_list_docs;
use crate::server::repo_scope::{ResolvedRepo, run_on_resolved_local_repo};
use crate::server::session::WsSession;
use anyhow::anyhow;
use deve_core::ledger::node_meta;
use deve_core::protocol::ServerMessage;
use std::sync::Arc;

pub(super) async fn handle_file_rename(
    state: &Arc<AppState>,
    ch: &DualChannel,
    session: &mut WsSession,
    scope: &ResolvedRepo,
    src_path: &str,
    dst_path: &str,
    src_file: &std::path::Path,
) {
    let doc_id = match state
        .repo
        .get_docid_in_local_repo(&scope.repo_name, src_path)
    {
        Ok(Some(doc_id)) => doc_id,
        Ok(None) => {
            ch.send_error(format!("Document not tracked: {}", src_path));
            return;
        }
        Err(e) => {
            ch.send_error(format!("Failed to resolve document: {}", e));
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
        ch.send_error(format!("Failed to rename file: {}", e));
        return;
    }
    if let Err(e) = state
        .sync_manager
        .persist_doc_in_local_repo(&scope.repo_name, doc_id)
    {
        tracing::error!("重命名投影持久化失败: {:?}", e);
        ch.send_error(format!("Failed to materialize renamed file: {}", e));
        return;
    }
    if let Err(e) = std::fs::remove_file(src_file) {
        tracing::error!("旧路径清理失败 {}: {:?}", src_path, e);
        ch.send_error(format!("Failed to remove old file: {}", e));
        return;
    }
    if let Ok((node_id, meta)) = run_on_resolved_local_repo(state, scope, |db| {
        let node_id = node_meta::get_node_id(db, dst_path)?
            .ok_or_else(|| anyhow!("Node not found: {}", dst_path))?;
        let meta =
            node_meta::get_node_meta(db, node_id)?.ok_or_else(|| anyhow!("Node meta missing"))?;
        Ok((node_id, meta))
    }) {
        if let Err(e) =
            broadcast_parent_dirs(state, ch, scope.repo_id, &scope.repo_name, meta.parent_id)
        {
            tracing::error!("广播父目录失败: {:?}", e);
        }
        let delta = state.tree_manager.with_tree_mut(scope.repo_id, |tm| {
            tm.update_node(
                node_id,
                meta.parent_id,
                meta.name.clone(),
                meta.path.clone(),
            )
        });
        ch.unicast(ServerMessage::TreeUpdate(delta));
    }
    handle_list_docs(state, ch, session).await;
    notify_fs_refresh(ch, scope.repo_id, dst_path, "renamed");
}
