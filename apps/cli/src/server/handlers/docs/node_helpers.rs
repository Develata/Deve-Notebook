// apps/cli/src/server/handlers/docs/node_helpers.rs
//! # Node Tree 辅助逻辑

use crate::server::AppState;
use crate::server::channel::DualChannel;
use anyhow::Result;
use deve_core::ledger::node_meta;
use deve_core::models::{NodeId, NodeKind, NodeMeta};
use deve_core::protocol::ServerMessage;
use std::sync::Arc;

pub fn broadcast_dir_chain(state: &Arc<AppState>, ch: &DualChannel, node_id: NodeId) -> Result<()> {
    let chain = state
        .repo
        .run_on_local_repo(state.repo.local_repo_name(), |db| {
            collect_dir_chain(db, node_id)
        })?;

    let mut tm = state
        .tree_manager
        .write()
        .unwrap_or_else(|e| e.into_inner());
    for (id, meta) in chain.into_iter().rev() {
        if tm.has_node(id) {
            continue;
        }
        let delta = tm.add_folder(id, meta.path.clone(), meta.parent_id, meta.name.clone());
        ch.broadcast(ServerMessage::TreeUpdate(delta));
    }
    Ok(())
}

pub fn broadcast_parent_dirs(
    state: &Arc<AppState>,
    ch: &DualChannel,
    parent_id: Option<NodeId>,
) -> Result<()> {
    if let Some(parent_id) = parent_id {
        broadcast_dir_chain(state, ch, parent_id)?;
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
