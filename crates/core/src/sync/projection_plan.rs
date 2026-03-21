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
    let mut dirs = HashSet::from([String::new()]);
    let mut docs = HashMap::new();
    let mut cache = HashMap::new();
    let mut visiting = HashSet::new();

    for (&node_id, meta) in &nodes {
        let path = build_path(node_id, &nodes, &mut cache, &mut visiting)?;
        if crate::utils::notegit::is_internal_repo_path(&path) {
            continue;
        }
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
    let mut nodes = HashMap::new();

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
                nodes.insert(
                    node_id,
                    ProjectionNode {
                        kind: NodeKind::File,
                        parent_id,
                        name,
                        doc_id: Some(doc_id),
                    },
                );
            }
            crate::models::StructureOp::CreateDir {
                node_id,
                parent_id,
                name,
            } => {
                nodes.insert(
                    node_id,
                    ProjectionNode {
                        kind: NodeKind::Dir,
                        parent_id,
                        name,
                        doc_id: None,
                    },
                );
            }
            crate::models::StructureOp::RenameNode {
                node_id, new_name, ..
            } => {
                let node = nodes.get_mut(&node_id).ok_or_else(|| {
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
                let node = nodes.get_mut(&node_id).ok_or_else(|| {
                    anyhow!(
                        "Structure projection move references missing node {}",
                        node_id
                    )
                })?;
                node.parent_id = new_parent_id;
            }
            crate::models::StructureOp::DeleteNode { node_id, .. } => {
                remove_subtree(&mut nodes, node_id);
            }
        }
    }

    Ok(nodes)
}

fn remove_subtree(nodes: &mut HashMap<NodeId, ProjectionNode>, root: NodeId) {
    let mut stack = vec![root];
    while let Some(node_id) = stack.pop() {
        let children = nodes
            .iter()
            .filter_map(|(child_id, node)| (node.parent_id == Some(node_id)).then_some(*child_id))
            .collect::<Vec<_>>();
        stack.extend(children);
        nodes.remove(&node_id);
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
                if !nodes.contains_key(&parent_id) {
                    anyhow::bail!(
                        "Structure projection references missing parent {} for node {}",
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
