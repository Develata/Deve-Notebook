// apps/cli/src/server/handlers/docs/node_helpers.rs
//! # Node Tree 辅助逻辑

use crate::server::AppState;
use crate::server::channel::DualChannel;
use anyhow::Result;
use deve_core::ledger::node_meta;
use deve_core::models::{NodeId, NodeKind, NodeMeta, RepoId};
use deve_core::protocol::ServerMessage;
use std::sync::Arc;

pub fn broadcast_dir_chain(
    state: &Arc<AppState>,
    ch: &DualChannel,
    repo_id: RepoId,
    repo_name: &str,
    node_id: NodeId,
) -> Result<()> {
    let chain = state
        .repo
        .run_on_local_repo(repo_name, |db| collect_dir_chain(db, node_id))?;

    for (id, meta) in chain.into_iter().rev() {
        let delta = state.tree_manager.with_tree_mut(repo_id, |tm| {
            (!tm.has_node(id))
                .then(|| tm.add_folder(id, meta.path.clone(), meta.parent_id, meta.name.clone()))
        });
        if let Some(delta) = delta {
            ch.unicast(ServerMessage::TreeUpdate {
                repo_id: Some(repo_id),
                delta,
            });
        }
    }
    Ok(())
}

pub fn broadcast_parent_dirs(
    state: &Arc<AppState>,
    ch: &DualChannel,
    repo_id: RepoId,
    repo_name: &str,
    parent_id: Option<NodeId>,
) -> Result<()> {
    if let Some(parent_id) = parent_id {
        broadcast_dir_chain(state, ch, repo_id, repo_name, parent_id)?;
    }
    Ok(())
}

fn collect_dir_chain(db: &redb::Database, node_id: NodeId) -> Result<Vec<(NodeId, NodeMeta)>> {
    let mut chain = Vec::new();
    let mut current = Some(node_id);

    while let Some(id) = current {
        let meta = node_meta::get_node_meta(db, id)?
            .ok_or_else(|| anyhow::anyhow!("Node meta missing"))?;
        if meta.kind == NodeKind::Dir {
            chain.push((id, meta.clone()));
            current = meta.parent_id;
        } else {
            current = meta.parent_id;
        }
    }

    Ok(chain)
}
