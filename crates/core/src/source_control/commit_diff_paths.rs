//! plan_ref:
//!   - 04_repository#tree-projection-contract
//!   - 05_diff_logic#source-control-runtime

use crate::ledger::schema::LEDGER_OPS;
use crate::models::{DocId, LedgerEvent, NodeId, StructureOp, deserialize_ledger_entry};
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
    replay_structure_until(db, seq, &mut structure)?;
    projected_doc_paths(&structure)
}

fn projected_doc_paths(structure: &StructureState) -> CommitDiffResult<HashMap<DocId, String>> {
    let mut cache = HashMap::<NodeId, String>::new();
    let mut visiting = HashSet::<NodeId>::new();
    let mut occupied_paths = HashMap::<String, NodeId>::new();
    let mut paths = HashMap::<DocId, String>::new();
    for (node_id, state) in &structure.nodes {
        let path = path_for(*node_id, &structure.nodes, &mut cache, &mut visiting)?;
        if !path.is_empty()
            && let Some(existing) = occupied_paths.insert(path.clone(), *node_id)
        {
            return Err(CommitDiffError::StructurePathCollision {
                path,
                existing,
                node_id: *node_id,
            });
        }
        if let Some(doc_id) = state.doc_id
            && let Some(existing) = paths.insert(doc_id, path.clone())
        {
            return Err(CommitDiffError::MultipleLivePaths {
                doc_id,
                existing,
                path,
            });
        }
    }
    Ok(paths)
}

fn replay_structure_until(
    db: &Database,
    seq: u64,
    structure: &mut StructureState,
) -> CommitDiffResult<()> {
    if seq == 0 {
        return Ok(());
    }
    let end = seq.saturating_add(1);
    let map_error = |message: String| CommitDiffError::LedgerRange {
        start: 1,
        end,
        message,
    };
    let read = db.begin_read().map_err(|err| map_error(err.to_string()))?;
    let table = read
        .open_table(LEDGER_OPS)
        .map_err(|err| map_error(err.to_string()))?;
    let range = table
        .range(1..=seq)
        .map_err(|err| map_error(err.to_string()))?;
    for item in range {
        let (key, value) = item.map_err(|err| map_error(err.to_string()))?;
        let entry = deserialize_ledger_entry(value.value()).map_err(|err| {
            map_error(format!(
                "failed to decode structure replay op at seq {}: {err}",
                key.value()
            ))
        })?;
        let LedgerEvent::Structure(op) = entry.event else {
            continue;
        };
        structure.apply(op)?;
    }
    Ok(())
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
                if node_id != NodeId::from_doc_id(doc_id) {
                    return Err(CommitDiffError::FileNodeDocMismatch { node_id, doc_id });
                }
                self.insert(
                    node_id,
                    NodeState {
                        name,
                        parent_id,
                        doc_id: Some(doc_id),
                    },
                )?;
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
                )?;
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

    fn insert(&mut self, node_id: NodeId, state: NodeState) -> CommitDiffResult<()> {
        if self.nodes.contains_key(&node_id) {
            return Err(CommitDiffError::DuplicateStructureNode { node_id });
        }
        let parent_id = state.parent_id;
        self.nodes.insert(node_id, state);
        self.add_child(parent_id, node_id);
        Ok(())
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
        Some(parent_id) => {
            let Some(parent) = nodes.get(&parent_id) else {
                visiting.remove(&node_id);
                return Err(CommitDiffError::StructureMissingNode { node_id: parent_id });
            };
            if parent.doc_id.is_some() {
                visiting.remove(&node_id);
                return Err(CommitDiffError::StructureParentNotDirectory { node_id, parent_id });
            }
            let parent_path = path_for(parent_id, nodes, cache, visiting)?;
            if parent_path.is_empty() {
                state.name.clone()
            } else {
                format!("{}/{}", parent_path, state.name)
            }
        }
        None => state.name.clone(),
    };
    visiting.remove(&node_id);
    cache.insert(node_id, path.clone());
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::{NodeState, StructureState, path_for, projected_doc_paths};
    use crate::models::{DocId, NodeId, StructureOp};
    use crate::source_control::CommitDiffError;
    use std::collections::{HashMap, HashSet};

    #[test]
    fn commit_diff_structure_delete_uses_children_index() {
        let root = NodeId::new();
        let moved_parent = NodeId::new();
        let child = NodeId::new();
        let grandchild_doc = DocId::new();
        let grandchild = NodeId::from_doc_id(grandchild_doc);
        let survivor_doc = DocId::new();
        let survivor = NodeId::from_doc_id(survivor_doc);
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
                doc_id: grandchild_doc,
                parent_id: Some(child),
                name: "nested.md".into(),
            },
            StructureOp::CreateFile {
                node_id: survivor,
                doc_id: survivor_doc,
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

    #[test]
    fn commit_diff_rejects_duplicate_live_structure_node() {
        let node_id = NodeId::new();
        let mut state = StructureState::default();
        state
            .apply(StructureOp::CreateDir {
                node_id,
                parent_id: None,
                name: "first".into(),
            })
            .expect("first create");
        let error = state
            .apply(StructureOp::CreateDir {
                node_id,
                parent_id: None,
                name: "replacement".into(),
            })
            .expect_err("duplicate live create must fail closed");
        assert_eq!(error, CommitDiffError::DuplicateStructureNode { node_id });
    }

    #[test]
    fn commit_diff_rejects_file_node_doc_id_mismatch() {
        let node_id = NodeId::new();
        let doc_id = DocId::new();
        let mut state = StructureState::default();
        let error = state
            .apply(StructureOp::CreateFile {
                node_id,
                doc_id,
                parent_id: None,
                name: "note.md".into(),
            })
            .expect_err("mismatched file identity must fail closed");
        assert_eq!(
            error,
            CommitDiffError::FileNodeDocMismatch { node_id, doc_id }
        );
    }

    #[test]
    fn commit_diff_rejects_missing_or_non_directory_parent() {
        let node_id = NodeId::new();
        let missing_parent = NodeId::new();
        let mut nodes = HashMap::from([(
            node_id,
            NodeState {
                name: "child".into(),
                parent_id: Some(missing_parent),
                doc_id: None,
            },
        )]);
        let error = path_for(node_id, &nodes, &mut HashMap::new(), &mut HashSet::new())
            .expect_err("missing parent must fail closed");
        assert_eq!(
            error,
            CommitDiffError::StructureMissingNode {
                node_id: missing_parent
            }
        );

        let parent_id = NodeId::from_doc_id(DocId::new());
        nodes.insert(
            parent_id,
            NodeState {
                name: "file.md".into(),
                parent_id: None,
                doc_id: Some(DocId::from_u128(parent_id.as_u128())),
            },
        );
        nodes.get_mut(&node_id).expect("child").parent_id = Some(parent_id);
        let error = path_for(node_id, &nodes, &mut HashMap::new(), &mut HashSet::new())
            .expect_err("file parent must fail closed");
        assert_eq!(
            error,
            CommitDiffError::StructureParentNotDirectory { node_id, parent_id }
        );
    }

    #[test]
    fn commit_diff_rejects_structure_path_collision() {
        let first = NodeId::new();
        let second = NodeId::new();
        let mut state = StructureState::default();
        for node_id in [first, second] {
            state
                .apply(StructureOp::CreateDir {
                    node_id,
                    parent_id: None,
                    name: "same".into(),
                })
                .expect("distinct node create");
        }

        let error = projected_doc_paths(&state).expect_err("path collision must fail closed");
        assert!(matches!(
            error,
            CommitDiffError::StructurePathCollision { path, existing, node_id }
                if path == "same"
                    && ((existing == first && node_id == second)
                        || (existing == second && node_id == first))
        ));
    }
}
