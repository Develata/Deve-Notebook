//! plan_ref:
//!   - 03_storage/repair#backup-export
//!   - 04_repository#tree-projection-contract
//!
use super::{LegacyLedgerEntry, doc};
use anyhow::{Context, Result, anyhow, bail};
use deve_core::models::{DocId, LedgerEvent, NodeId, Op, StructureOp};
use std::collections::{HashMap, HashSet};
use std::path::{Component, Path, PathBuf};

#[derive(Clone)]
struct LegacyNode {
    parent_id: Option<NodeId>,
    name: String,
    doc_id: Option<DocId>,
    deleted: bool,
}

pub(super) fn write(
    output: Option<String>,
    selected_doc: Option<String>,
    entries: &[(u64, LegacyLedgerEntry)],
) -> Result<()> {
    let mut nodes = HashMap::new();
    let mut content_ops: HashMap<DocId, Vec<Op>> = HashMap::new();
    for (_global_seq, entry) in entries {
        match &entry.event {
            LedgerEvent::Content(op) => {
                let doc_id = entry
                    .doc_id
                    .ok_or_else(|| anyhow!("legacy content fact is missing doc_id"))?;
                content_ops.entry(doc_id).or_default().push(op.clone());
            }
            LedgerEvent::Structure(op) => apply_structure(&mut nodes, op)?,
            LedgerEvent::MergeAnchor(_) => {}
        }
    }

    if let Some(selected_doc) = selected_doc {
        let doc_id = doc::parse_doc_id(&selected_doc)?;
        let ops = content_ops.get(&doc_id).map(Vec::as_slice).unwrap_or(&[]);
        let content = deve_core::state::try_apply_content_ops("", ops)
            .ok_or_else(|| anyhow!("legacy content facts for {doc_id} are invalid"))?;
        return doc::write_markdown_file(&doc::output_file(output)?, &content);
    }

    let output_dir = PathBuf::from(output.unwrap_or_else(|| "export-v2".into()));
    let mut documents = Vec::new();
    let mut seen_paths = HashSet::new();
    for (node_id, node) in &nodes {
        let Some(doc_id) = node.doc_id else {
            continue;
        };
        let Some(path) = node_path(*node_id, &nodes, &mut HashSet::new())? else {
            continue;
        };
        if !seen_paths.insert(path.clone()) {
            bail!("legacy structure facts resolve multiple documents to {path}");
        }
        let ops = content_ops.get(&doc_id).map(Vec::as_slice).unwrap_or(&[]);
        let content = deve_core::state::try_apply_content_ops("", ops)
            .ok_or_else(|| anyhow!("legacy content facts for {doc_id} are invalid"))?;
        documents.push((path, content));
    }
    documents.sort_by(|left, right| left.0.cmp(&right.0));
    let exported = u32::try_from(documents.len()).context("too many legacy documents to export")?;
    for (path, content) in documents {
        doc::write_markdown_file(&output_dir.join(path), &content)?;
    }
    println!(
        "Exported {exported} markdown files from schema v2 to {:?}",
        output_dir
    );
    Ok(())
}

fn apply_structure(nodes: &mut HashMap<NodeId, LegacyNode>, op: &StructureOp) -> Result<()> {
    match op {
        StructureOp::CreateFile {
            node_id,
            doc_id,
            parent_id,
            name,
        } => {
            nodes.insert(
                *node_id,
                LegacyNode {
                    parent_id: *parent_id,
                    name: validate_name(name)?,
                    doc_id: Some(*doc_id),
                    deleted: false,
                },
            );
        }
        StructureOp::CreateDir {
            node_id,
            parent_id,
            name,
        } => {
            nodes.insert(
                *node_id,
                LegacyNode {
                    parent_id: *parent_id,
                    name: validate_name(name)?,
                    doc_id: None,
                    deleted: false,
                },
            );
        }
        StructureOp::RenameNode {
            node_id, new_name, ..
        } => {
            nodes
                .get_mut(node_id)
                .ok_or_else(|| anyhow!("legacy rename references missing node {node_id}"))?
                .name = validate_name(new_name)?;
        }
        StructureOp::MoveNode {
            node_id,
            new_parent_id,
            ..
        } => {
            nodes
                .get_mut(node_id)
                .ok_or_else(|| anyhow!("legacy move references missing node {node_id}"))?
                .parent_id = *new_parent_id;
        }
        StructureOp::DeleteNode { node_id, .. } => {
            nodes
                .get_mut(node_id)
                .ok_or_else(|| anyhow!("legacy delete references missing node {node_id}"))?
                .deleted = true;
        }
    }
    Ok(())
}

fn validate_name(name: &str) -> Result<String> {
    if name.is_empty()
        || Path::new(name)
            .components()
            .any(|part| !matches!(part, Component::Normal(_)))
    {
        bail!("unsafe legacy structure name: {name:?}");
    }
    Ok(name.to_string())
}

fn node_path(
    node_id: NodeId,
    nodes: &HashMap<NodeId, LegacyNode>,
    visiting: &mut HashSet<NodeId>,
) -> Result<Option<String>> {
    if !visiting.insert(node_id) {
        bail!("legacy structure cycle detected at node {node_id}");
    }
    let node = nodes
        .get(&node_id)
        .ok_or_else(|| anyhow!("legacy structure references missing node {node_id}"))?;
    if node.deleted {
        visiting.remove(&node_id);
        return Ok(None);
    }
    let path = if let Some(parent_id) = node.parent_id {
        let Some(parent) = node_path(parent_id, nodes, visiting)? else {
            visiting.remove(&node_id);
            return Ok(None);
        };
        format!("{parent}/{}", node.name)
    } else {
        node.name.clone()
    };
    visiting.remove(&node_id);
    Ok(Some(path))
}
