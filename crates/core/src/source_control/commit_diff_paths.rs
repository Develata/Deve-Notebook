//! plan_ref:
//!   - 04_repository#tree-projection-contract
//!   - 05_diff_logic#source-control-runtime

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

#[derive(Default)]
struct StructureState {
    nodes: HashMap<NodeId, NodeState>,
    children_by_parent: HashMap<NodeId, HashSet<NodeId>>,
}

pub(super) fn doc_paths_at_seq(
    db: &Database,
    seq: u64,
) -> CommitDiffResult<HashMap<DocId, String>> {
    let mut structure = StructureState::default();
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
        structure.apply(op)?;
    }
    let mut cache = HashMap::<NodeId, String>::new();
    let mut visiting = HashSet::<NodeId>::new();
    let mut paths = HashMap::<DocId, String>::new();
    for (node_id, state) in &structure.nodes {
        if let Some(doc_id) = state.doc_id {
            let path = path_for(*node_id, &structure.nodes, &mut cache, &mut visiting)?;
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

impl StructureState {
    fn apply(&mut self, op: StructureOp) -> CommitDiffResult<()> {
        match op {
            StructureOp::CreateFile {
                node_id,
                doc_id,
                parent_id,
                name,
            } => {
                self.insert(
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
                self.insert(
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
                let state = self
                    .nodes
                    .get_mut(&node_id)
                    .ok_or(CommitDiffError::RenameMissingNode { node_id })?;
                state.name = new_name;
            }
            StructureOp::MoveNode {
                node_id,
                new_parent_id,
                ..
            } => {
                self.move_node(node_id, new_parent_id)?;
            }
            StructureOp::DeleteNode { node_id, .. } => self.remove_subtree(node_id),
        }
        Ok(())
    }

    fn insert(&mut self, node_id: NodeId, state: NodeState) {
        if let Some(previous) = self.nodes.insert(node_id, state) {
            self.remove_child(previous.parent_id, node_id);
        }
        let parent_id = self.nodes.get(&node_id).and_then(|node| node.parent_id);
        self.add_child(parent_id, node_id);
    }

    fn move_node(
        &mut self,
        node_id: NodeId,
        new_parent_id: Option<NodeId>,
    ) -> CommitDiffResult<()> {
        let old_parent_id = self
            .nodes
            .get(&node_id)
            .ok_or(CommitDiffError::MoveMissingNode { node_id })?
            .parent_id;
        self.remove_child(old_parent_id, node_id);
        self.nodes
            .get_mut(&node_id)
            .expect("node existence checked above")
            .parent_id = new_parent_id;
        self.add_child(new_parent_id, node_id);
        Ok(())
    }

    fn remove_subtree(&mut self, root: NodeId) {
        let root_parent = self.nodes.get(&root).and_then(|node| node.parent_id);
        self.remove_child(root_parent, root);
        let mut stack = vec![root];
        while let Some(node_id) = stack.pop() {
            if let Some(children) = self.children_by_parent.remove(&node_id) {
                stack.extend(children);
            }
            self.nodes.remove(&node_id);
        }
    }

    fn add_child(&mut self, parent_id: Option<NodeId>, child_id: NodeId) {
        if let Some(parent_id) = parent_id {
            self.children_by_parent
                .entry(parent_id)
                .or_default()
                .insert(child_id);
        }
    }

    fn remove_child(&mut self, parent_id: Option<NodeId>, child_id: NodeId) {
        let Some(parent_id) = parent_id else {
            return;
        };
        let remove_bucket = self
            .children_by_parent
            .get_mut(&parent_id)
            .is_some_and(|children| {
                children.remove(&child_id);
                children.is_empty()
            });
        if remove_bucket {
            self.children_by_parent.remove(&parent_id);
        }
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

#[cfg(test)]
mod tests {
    use super::StructureState;
    use crate::models::{DocId, NodeId, StructureOp};

    #[test]
    fn commit_diff_structure_delete_uses_children_index() {
        let root = NodeId::new();
        let moved_parent = NodeId::new();
        let child = NodeId::new();
        let grandchild = NodeId::new();
        let survivor = NodeId::new();
        let mut state = StructureState::default();
        for op in [
            StructureOp::CreateDir {
                node_id: root,
                parent_id: None,
                name: "root".into(),
            },
            StructureOp::CreateDir {
                node_id: moved_parent,
                parent_id: None,
                name: "other".into(),
            },
            StructureOp::CreateDir {
                node_id: child,
                parent_id: Some(root),
                name: "child".into(),
            },
            StructureOp::CreateFile {
                node_id: grandchild,
                doc_id: DocId::new(),
                parent_id: Some(child),
                name: "nested.md".into(),
            },
            StructureOp::CreateFile {
                node_id: survivor,
                doc_id: DocId::new(),
                parent_id: Some(root),
                name: "survivor.md".into(),
            },
            StructureOp::MoveNode {
                node_id: survivor,
                doc_id: None,
                new_parent_id: Some(moved_parent),
            },
            StructureOp::DeleteNode {
                node_id: root,
                doc_id: None,
            },
        ] {
            state.apply(op).expect("structure op");
        }
        assert!(!state.nodes.contains_key(&root));
        assert!(!state.nodes.contains_key(&child));
        assert!(!state.nodes.contains_key(&grandchild));
        assert!(state.nodes.contains_key(&moved_parent));
        assert!(state.nodes.contains_key(&survivor));
        assert_eq!(state.children_by_parent.len(), 1);
        assert!(state.children_by_parent[&moved_parent].contains(&survivor));
    }
}
