//! plan_ref:
//!   - 06_repository#tree-projection-contract
//!   - 07_diff_logic#source-control-runtime

use crate::ledger::range;
use crate::models::{DocId, LedgerEvent, NodeId, StructureOp};
use crate::source_control::commit_diff_error::{CommitDiffError, CommitDiffResult};
use redb::Database;
use std::collections::{HashMap, HashSet};

struct NodeState {
    name: String,
    parent_id: Option<NodeId>,
    doc_id: Option<DocId>,
}

pub(super) fn doc_paths_at_seq(
    db: &Database,
    seq: u64,
) -> CommitDiffResult<HashMap<DocId, String>> {
    let mut nodes = HashMap::<NodeId, NodeState>::new();
    let end = seq.saturating_add(1);
    for (_, entry) in
        range::get_ops_in_range(db, 1, end).map_err(|err| CommitDiffError::LedgerRange {
            start: 1,
            end,
            message: err.to_string(),
        })?
    {
        let LedgerEvent::Structure(op) = entry.event else {
            continue;
        };
        apply_structure(&mut nodes, op)?;
    }
    let mut cache = HashMap::<NodeId, String>::new();
    let mut visiting = HashSet::<NodeId>::new();
    let mut paths = HashMap::<DocId, String>::new();
    for (node_id, state) in &nodes {
        if let Some(doc_id) = state.doc_id {
            let path = path_for(*node_id, &nodes, &mut cache, &mut visiting)?;
            if let Some(existing) = paths.insert(doc_id, path.clone()) {
                return Err(CommitDiffError::MultipleLivePaths {
                    doc_id,
                    existing,
                    path,
                });
            }
        }
    }
    Ok(paths)
}

fn apply_structure(
    nodes: &mut HashMap<NodeId, NodeState>,
    op: StructureOp,
) -> CommitDiffResult<()> {
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
            let state = nodes
                .get_mut(&node_id)
                .ok_or(CommitDiffError::RenameMissingNode { node_id })?;
            state.name = new_name;
        }
        StructureOp::MoveNode {
            node_id,
            new_parent_id,
            ..
        } => {
            let state = nodes
                .get_mut(&node_id)
                .ok_or(CommitDiffError::MoveMissingNode { node_id })?;
            state.parent_id = new_parent_id;
        }
        StructureOp::DeleteNode { node_id, .. } => remove_subtree(nodes, node_id),
    }
    Ok(())
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
    visiting: &mut HashSet<NodeId>,
) -> CommitDiffResult<String> {
    if let Some(path) = cache.get(&node_id) {
        return Ok(path.clone());
    }
    if !visiting.insert(node_id) {
        return Err(CommitDiffError::StructureCycle { node_id });
    }
    let Some(state) = nodes.get(&node_id) else {
        visiting.remove(&node_id);
        return Err(CommitDiffError::StructureMissingNode { node_id });
    };
    let path = match state.parent_id {
        Some(parent_id) if nodes.contains_key(&parent_id) => {
            let parent_path = path_for(parent_id, nodes, cache, visiting)?;
            if parent_path.is_empty() {
                state.name.clone()
            } else {
                format!("{}/{}", parent_path, state.name)
            }
        }
        _ => state.name.clone(),
    };
    visiting.remove(&node_id);
    cache.insert(node_id, path.clone());
    Ok(path)
}
