use crate::server::AppState;
use crate::server::channel::DualChannel;
use crate::server::handlers::docs::node_helpers::broadcast_parent_dirs;
use crate::server::repo_scope::{ResolvedRepo, local_repo_path, run_on_resolved_local_repo};
use anyhow::{Result, anyhow};
use deve_core::ledger::node_meta;
use deve_core::models::{DocId, NodeId};
use deve_core::protocol::ServerMessage;
use deve_core::state;
use deve_core::sync::reconcile;
use std::sync::Arc;

pub(super) fn create_file_from_content(
    state: &Arc<AppState>,
    scope: &ResolvedRepo,
    rel_path: &str,
    content: &str,
    peer_label: &str,
) -> Result<DocId> {
    let path = local_repo_path(state, scope, rel_path)?;
    if path.exists() {
        anyhow::bail!("Target file already exists on disk: {}", rel_path);
    }
    if state
        .repo
        .get_tracked_docid_in_local_repo(&scope.repo_name, rel_path)?
        .is_some()
    {
        anyhow::bail!("Target already tracked: {}", rel_path);
    }
    let doc_id = state.repo.apply_file_structure_in_local_repo(
        &scope.repo_name,
        rel_path,
        None,
        peer_label,
    )?;
    if !content.is_empty() {
        reconcile::append_patch_in_local_repo(
            state.repo.as_ref(),
            &scope.repo_name,
            doc_id,
            peer_label,
            &state::compute_diff("", content),
        )?;
    }
    state
        .sync_manager
        .persist_doc_in_local_repo(&scope.repo_name, doc_id)?;
    Ok(doc_id)
}

pub(super) fn broadcast_file_tree_update(
    state: &Arc<AppState>,
    ch: &DualChannel,
    scope: &ResolvedRepo,
    doc_id: DocId,
    scope_nonce: u64,
) {
    let node_id = NodeId::from_doc_id(doc_id);
    if let Ok(meta) = run_on_resolved_local_repo(state, scope, |db| {
        node_meta::get_node_meta(db, node_id)
            .and_then(|meta| meta.ok_or_else(|| anyhow!("File node meta missing")))
    }) {
        if let Err(e) = broadcast_parent_dirs(
            state,
            ch,
            scope.repo_id,
            &scope.repo_name,
            meta.parent_id,
            scope_nonce,
        ) {
            tracing::error!("广播父目录失败: {:?}", e);
        }
        let delta = state.tree_manager.with_tree_mut(scope.repo_id, None, |tm| {
            tm.add_file(
                node_id,
                meta.path.clone(),
                meta.parent_id,
                meta.name.clone(),
                doc_id,
            )
        });
        ch.unicast(ServerMessage::TreeUpdate {
            request_id: None,
            repo_id: Some(scope.repo_id),
            branch: None,
            scope_nonce: Some(scope_nonce),
            delta,
        });
    }
}
