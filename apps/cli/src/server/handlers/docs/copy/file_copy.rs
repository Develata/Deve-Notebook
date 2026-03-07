use crate::server::AppState;
use crate::server::channel::DualChannel;
use crate::server::handlers::docs::node_helpers::broadcast_parent_dirs;
use crate::server::repo_scope::{ResolvedRepo, run_on_resolved_local_repo};
use anyhow::anyhow;
use deve_core::ledger::node_meta;
use deve_core::models::NodeId;
use deve_core::protocol::ServerMessage;
use std::path::Path;
use std::sync::Arc;

pub(super) fn copy_file(
    state: &Arc<AppState>,
    ch: &DualChannel,
    scope: &ResolvedRepo,
    src: &Path,
    dst: &Path,
    src_path: &str,
    dest_path: &str,
) {
    if let Err(e) = std::fs::copy(src, dst) {
        tracing::error!("复制失败 {} -> {:?}: {:?}", src_path, dst, e);
        ch.send_error(format!("Copy failed: {}", e));
        return;
    }

    let Ok(doc_id) = state
        .repo
        .create_docid_in_local_repo(&scope.repo_name, dest_path)
    else {
        tracing::error!("Ledger 注册复制文档失败");
        return;
    };
    tracing::info!("已复制 {} -> {} (DocId: {})", src_path, dest_path, doc_id);
    let node_id = NodeId::from_doc_id(doc_id);
    let Ok(meta) = run_on_resolved_local_repo(state, scope, |db| {
        node_meta::get_node_meta(db, node_id)
            .and_then(|m| m.ok_or_else(|| anyhow!("File node meta missing")))
    }) else {
        return;
    };
    if let Err(e) =
        broadcast_parent_dirs(state, ch, scope.repo_id, &scope.repo_name, meta.parent_id)
    {
        tracing::error!("广播父目录失败: {:?}", e);
    }
    let delta = state.tree_manager.with_tree_mut(scope.repo_id, |tm| {
        tm.add_file(
            node_id,
            meta.path.clone(),
            meta.parent_id,
            meta.name.clone(),
            doc_id,
        )
    });
    ch.unicast(ServerMessage::TreeUpdate(delta));
}
