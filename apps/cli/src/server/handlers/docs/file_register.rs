use crate::server::AppState;
use crate::server::channel::DualChannel;
use crate::server::handlers::docs::node_helpers::broadcast_parent_dirs;
use crate::server::repo_scope::{ResolvedRepo, run_on_resolved_local_repo};
use anyhow::{Result, anyhow};
use deve_core::ledger::node_meta;
use deve_core::models::{DocId, NodeId};
use deve_core::protocol::ServerMessage;
use deve_core::state;
use deve_core::sync::reconcile;
use std::path::Path;
use std::sync::Arc;

pub(super) fn register_file_from_disk(
    state: &Arc<AppState>,
    scope: &ResolvedRepo,
    path: &Path,
    rel_path: &str,
    peer_label: &str,
) -> Result<DocId> {
    if path.exists() && !path.is_file() {
        anyhow::bail!("Target path is not a file: {}", rel_path);
    }
    let existing_doc_id = state
        .repo
        .get_docid_in_local_repo(&scope.repo_name, rel_path)?;
    let disk_content = if path.exists() {
        Some(std::fs::read_to_string(path)?)
    } else {
        None
    };
    let doc_id = state.repo.apply_file_structure_in_local_repo(
        &scope.repo_name,
        rel_path,
        existing_doc_id,
        peer_label,
    )?;
    if let Some(content) = disk_content {
        if existing_doc_id.is_none() && !content.is_empty() {
            reconcile::append_patch_in_local_repo(
                state.repo.as_ref(),
                &scope.repo_name,
                doc_id,
                peer_label,
                &state::compute_diff("", &content),
            )?;
        }
    } else {
        state
            .sync_manager
            .persist_doc_in_local_repo(&scope.repo_name, doc_id)?;
    }
    Ok(doc_id)
}

pub(super) fn broadcast_file_tree_update(
    state: &Arc<AppState>,
    ch: &DualChannel,
    scope: &ResolvedRepo,
    doc_id: DocId,
) {
    let node_id = NodeId::from_doc_id(doc_id);
    if let Ok(meta) = run_on_resolved_local_repo(state, scope, |db| {
        node_meta::get_node_meta(db, node_id)
            .and_then(|meta| meta.ok_or_else(|| anyhow!("File node meta missing")))
    }) {
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
}
