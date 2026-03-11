use crate::ledger::range;
use crate::models::{DocId, LedgerEvent, NodeId, StructureOp};
use anyhow::Result;
use redb::Database;
use std::collections::HashMap;

struct NodeState {
    name: String,
    parent_id: Option<NodeId>,
    doc_id: Option<DocId>,
}

pub(super) fn doc_paths_at_seq(db: &Database, seq: u64) -> Result<HashMap<DocId, String>> {
    let mut nodes = HashMap::<NodeId, NodeState>::new();
    for (_, entry) in range::get_ops_in_range(db, 1, seq.saturating_add(1))? {
        let LedgerEvent::Structure(op) = entry.event else {
            continue;
        };
        apply_structure(&mut nodes, op);
    }
    let mut cache = HashMap::<NodeId, String>::new();
    let mut paths = HashMap::<DocId, String>::new();
    for (node_id, state) in &nodes {
        if let Some(doc_id) = state.doc_id {
            paths.insert(doc_id, path_for(*node_id, &nodes, &mut cache));
        }
    }
    Ok(paths)
}

fn apply_structure(nodes: &mut HashMap<NodeId, NodeState>, op: StructureOp) {
    match op {
        StructureOp::CreateFile {
            node_id,
            doc_id,
            parent_id,
            name,
        } => {
            nodes.insert(
                node_id,
                NodeState {
                    name,
                    parent_id,
                    doc_id: Some(doc_id),
                },
            );
        }
        StructureOp::CreateDir {
            node_id,
            parent_id,
            name,
        } => {
            nodes.insert(
                node_id,
                NodeState {
                    name,
                    parent_id,
                    doc_id: None,
                },
            );
        }
        StructureOp::RenameNode {
            node_id, new_name, ..
        } => {
            if let Some(state) = nodes.get_mut(&node_id) {
                state.name = new_name;
            }
        }
        StructureOp::MoveNode {
            node_id,
            new_parent_id,
            ..
        } => {
            if let Some(state) = nodes.get_mut(&node_id) {
                state.parent_id = new_parent_id;
            }
        }
        StructureOp::DeleteNode { node_id, .. } => remove_subtree(nodes, node_id),
    }
}

fn remove_subtree(nodes: &mut HashMap<NodeId, NodeState>, root: NodeId) {
    let mut stack = vec![root];
    while let Some(node_id) = stack.pop() {
        let children: Vec<_> = nodes
            .iter()
            .filter_map(|(child_id, state)| (state.parent_id == Some(node_id)).then_some(*child_id))
            .collect();
        stack.extend(children);
        nodes.remove(&node_id);
    }
}

fn path_for(
    node_id: NodeId,
    nodes: &HashMap<NodeId, NodeState>,
    cache: &mut HashMap<NodeId, String>,
) -> String {
    if let Some(path) = cache.get(&node_id) {
        return path.clone();
    }
    let Some(state) = nodes.get(&node_id) else {
        return String::new();
    };
    let path = match state.parent_id.and_then(|parent| nodes.get(&parent)) {
        Some(_) => {
            let parent_path = path_for(state.parent_id.expect("checked"), nodes, cache);
            if parent_path.is_empty() {
                state.name.clone()
            } else {
                format!("{}/{}", parent_path, state.name)
            }
        }
        None => state.name.clone(),
    };
    cache.insert(node_id, path.clone());
    path
}
