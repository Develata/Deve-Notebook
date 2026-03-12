use super::errors;
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

pub(super) async fn handle_dir_rename(
    state: &Arc<AppState>,
    ch: &DualChannel,
    session: &mut WsSession,
    scope: &ResolvedRepo,
    old_path: &str,
    dst_name: &str,
) {
    let node_id = match state.repo.apply_dir_rename_structure_in_local_repo(
        &scope.repo_name,
        old_path,
        dst_name,
        "local_rename",
    ) {
        Ok(Some(node_id)) => node_id,
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
    if let Ok((node_id, meta)) = run_on_resolved_local_repo(state, scope, |db| {
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
        ch.unicast(ServerMessage::TreeUpdate {
            repo_id: Some(scope.repo_id),
            delta,
        });
    }
    handle_list_docs(state, ch, session).await;
    notify_fs_refresh(ch, scope.repo_id, dst_name, "renamed");
}
