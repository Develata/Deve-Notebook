use super::notify_fs_refresh;
use crate::server::AppState;
use crate::server::channel::DualChannel;
use crate::server::handlers::docs::node_helpers::broadcast_parent_dirs;
use crate::server::handlers::listing::handle_list_docs;
use crate::server::repo_scope::{ResolvedRepo, local_repo_path, run_on_resolved_local_repo};
use crate::server::session::WsSession;
use anyhow::anyhow;
use deve_core::ledger::node_meta;
use deve_core::protocol::ServerMessage;
use std::sync::Arc;

pub(super) async fn handle_dir_rename(
    state: &Arc<AppState>,
    ch: &DualChannel,
    session: &mut WsSession,
    scope: &ResolvedRepo,
    old_path: &str,
    dst_name: &str,
    src: &std::path::Path,
) {
    let dst = match local_repo_path(state, scope, dst_name) {
        Ok(path) => path,
        Err(err) => {
            ch.send_error(err.to_string());
            return;
        }
    };
    if let Some(parent) = dst.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Err(e) = std::fs::rename(src, dst) {
        tracing::error!("重命名失败 {} -> {}: {:?}", old_path, dst_name, e);
        ch.send_error(format!("Failed to rename: {}", e));
        return;
    }
    if let Err(e) = state
        .repo
        .rename_folder_in_local_repo(&scope.repo_name, old_path, dst_name)
    {
        tracing::error!("Ledger 文件夹重命名失败: {:?}", e);
    }
    if let Ok((node_id, meta)) = run_on_resolved_local_repo(state, scope, |db| {
        let node_id = node_meta::get_node_id(db, dst_name)?
            .ok_or_else(|| anyhow!("Node not found: {}", dst_name))?;
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
    notify_fs_refresh(ch, scope.repo_id, dst_name, "renamed");
}
