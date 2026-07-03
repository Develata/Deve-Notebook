//! plan_ref:
//!   - 04_repository#tree-projection-contract

use crate::ledger::RepoManager;
use crate::ledger::schema::LEDGER_OPS;
use crate::models::{DocId, LedgerEvent, NodeId, NodeKind, deserialize_ledger_entry};
use anyhow::{Result, anyhow};
use redb::ReadableTable;
use std::collections::{HashMap, HashSet};
use std::path::Path;

pub(super) struct ProjectionPlan {
    pub dirs: HashSet<String>,
    pub docs: HashMap<String, DocId>,
}

pub(super) fn build(repo: &RepoManager, repo_name: &str) -> Result<ProjectionPlan> {
    let nodes = repo.run_on_local_repo(repo_name, build_structure_state)?;
    build_from_nodes(nodes)
}

pub(super) fn build_stem(repo: &RepoManager, repo_stem: &str) -> Result<ProjectionPlan> {
    let nodes = repo.run_on_local_repo_stem(repo_stem, build_structure_state)?;
    build_from_nodes(nodes)
}

fn build_from_nodes(nodes: HashMap<NodeId, ProjectionNode>) -> Result<ProjectionPlan> {
    let mut dirs = HashSet::from([String::new()]);
    let mut docs = HashMap::new();
    let mut cache = HashMap::new();
    let mut visiting = HashSet::new();

    let mut occupied_paths = HashMap::new();
    for (&node_id, meta) in &nodes {
        let path = build_path(node_id, &nodes, &mut cache, &mut visiting)?;
        if crate::utils::notegit::is_internal_repo_path(&path) {
            continue;
        }
        ensure_unique_path(&mut occupied_paths, node_id, &path)?;
        if meta.kind == NodeKind::Dir {
            if !path.is_empty() {
                dirs.insert(path.clone());
            }
            insert_parents(&mut dirs, &path);
            continue;
        }
        let Some(doc_id) = meta.doc_id else {
            return Err(anyhow!(
                "Structure projection lost doc identity for file node {}",
                node_id
            ));
        };
        insert_parents(&mut dirs, &path);
        docs.insert(path, doc_id);
    }

    Ok(ProjectionPlan { dirs, docs })
}

#[derive(Clone)]
struct ProjectionNode {
    kind: NodeKind,
    parent_id: Option<NodeId>,
    name: String,
    doc_id: Option<DocId>,
}

fn build_structure_state(db: &redb::Database) -> Result<HashMap<NodeId, ProjectionNode>> {
    let read = db.begin_read()?;
    let ops = read.open_table(LEDGER_OPS)?;
    let mut state = ProjectionState::default();

    for item in ops.iter()? {
        let (_, bytes) = item?;
        let entry = deserialize_ledger_entry(bytes.value())?;
        let LedgerEvent::Structure(op) = entry.event else {
            continue;
        };
        match op {
            crate::models::StructureOp::CreateFile {
                node_id,
                doc_id,
                parent_id,
                name,
            } => {
                if node_id != NodeId::from_doc_id(doc_id) {
                    return Err(anyhow!(
                        "Structure projection file node/doc mismatch for {}",
                        doc_id
                    ));
                }
                state.insert_node(
                    node_id,
                    ProjectionNode {
                        kind: NodeKind::File,
                        parent_id,
                        name,
                        doc_id: Some(doc_id),
                    },
                )?;
            }
            crate::models::StructureOp::CreateDir {
                node_id,
                parent_id,
                name,
            } => {
                state.insert_node(
                    node_id,
                    ProjectionNode {
                        kind: NodeKind::Dir,
                        parent_id,
                        name,
                        doc_id: None,
                    },
                )?;
            }
            crate::models::StructureOp::RenameNode {
                node_id, new_name, ..
            } => {
                let node = state.nodes.get_mut(&node_id).ok_or_else(|| {
                    anyhow!(
                        "Structure projection rename references missing node {}",
                        node_id
                    )
                })?;
                node.name = new_name;
            }
            crate::models::StructureOp::MoveNode {
                node_id,
                new_parent_id,
                ..
            } => {
                state.move_node(node_id, new_parent_id).ok_or_else(|| {
                    anyhow!(
                        "Structure projection move references missing node {}",
                        node_id
                    )
                })?;
            }
            crate::models::StructureOp::DeleteNode { node_id, .. } => {
                state.remove_subtree(node_id);
            }
        }
    }

    Ok(state.nodes)
}

#[derive(Default)]
struct ProjectionState {
    nodes: HashMap<NodeId, ProjectionNode>,
    children_by_parent: HashMap<NodeId, Vec<NodeId>>,
}

impl ProjectionState {
    fn insert_node(&mut self, node_id: NodeId, node: ProjectionNode) -> Result<()> {
        let parent_id = node.parent_id;
        if self.nodes.insert(node_id, node).is_some() {
            anyhow::bail!("Structure projection duplicate create for node {}", node_id);
        }
        self.attach_child(parent_id, node_id);
        Ok(())
    }

    fn move_node(&mut self, node_id: NodeId, new_parent_id: Option<NodeId>) -> Option<()> {
        let old_parent_id = self.nodes.get(&node_id)?.parent_id;
        self.detach_child(old_parent_id, node_id);
        self.attach_child(new_parent_id, node_id);
        self.nodes.get_mut(&node_id)?.parent_id = new_parent_id;
        Some(())
    }

    fn remove_subtree(&mut self, root: NodeId) {
        let parent_id = self.nodes.get(&root).and_then(|node| node.parent_id);
        self.detach_child(parent_id, root);

        let mut stack = vec![root];
        while let Some(node_id) = stack.pop() {
            if let Some(children) = self.children_by_parent.remove(&node_id) {
                stack.extend(children);
            }
            self.nodes.remove(&node_id);
        }
    }

    fn attach_child(&mut self, parent_id: Option<NodeId>, child_id: NodeId) {
        let Some(parent_id) = parent_id else {
            return;
        };
        let children = self.children_by_parent.entry(parent_id).or_default();
        if !children.contains(&child_id) {
            children.push(child_id);
        }
    }

    fn detach_child(&mut self, parent_id: Option<NodeId>, child_id: NodeId) {
        let Some(parent_id) = parent_id else {
            return;
        };
        let should_remove = if let Some(children) = self.children_by_parent.get_mut(&parent_id) {
            children.retain(|id| *id != child_id);
            children.is_empty()
        } else {
            false
        };
        if should_remove {
            self.children_by_parent.remove(&parent_id);
        }
    }
}

fn build_path(
    node_id: NodeId,
    nodes: &HashMap<NodeId, ProjectionNode>,
    cache: &mut HashMap<NodeId, String>,
    visiting: &mut HashSet<NodeId>,
) -> Result<String> {
    if let Some(path) = cache.get(&node_id) {
        return Ok(path.clone());
    }
    if !visiting.insert(node_id) {
        anyhow::bail!("Structure projection contains cycle at node {}", node_id);
    }
    let path = match nodes.get(&node_id) {
        Some(node) => match node.parent_id {
            Some(parent_id) => {
                let Some(parent_node) = nodes.get(&parent_id) else {
                    anyhow::bail!(
                        "Structure projection references missing parent {} for node {}",
                        parent_id,
                        node_id
                    );
                };
                if parent_node.kind != NodeKind::Dir {
                    anyhow::bail!(
                        "Structure projection parent is not a directory {} for node {}",
                        parent_id,
                        node_id
                    );
                }
                let parent = build_path(parent_id, nodes, cache, visiting)?;
                if parent.is_empty() {
                    node.name.clone()
                } else {
                    format!("{parent}/{}", node.name)
                }
            }
            None => node.name.clone(),
        },
        None => String::new(),
    };
    visiting.remove(&node_id);
    cache.insert(node_id, path.clone());
    Ok(path)
}

fn ensure_unique_path(
    occupied_paths: &mut HashMap<String, NodeId>,
    node_id: NodeId,
    path: &str,
) -> Result<()> {
    if path.is_empty() {
        return Ok(());
    }
    if let Some(existing) = occupied_paths.insert(path.to_string(), node_id) {
        anyhow::bail!(
            "Structure projection path collision for {} between {} and {}",
            path,
            existing,
            node_id
        );
    }
    Ok(())
}

fn insert_parents(dirs: &mut HashSet<String>, path: &str) {
    let mut cursor = Path::new(path).parent();
    while let Some(parent) = cursor {
        let value = crate::utils::path::to_forward_slash(&parent.to_string_lossy());
        if value.is_empty() || !dirs.insert(value.clone()) {
            break;
        }
        cursor = parent.parent();
    }
}
