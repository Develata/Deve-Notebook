//! plan_ref:
//!   - 03_storage#facts-partition
//!   - 03_storage#projection-contract
//!   - 04_repository#tree-projection-contract
//!
use crate::models::{DocId, NodeId, NodeKind, NodeMeta, StructureOp};
use anyhow::Result;
use redb::WriteTransaction;
use std::collections::HashSet;

use super::errors::reject_invalid_structure;
use super::projection::{
    child_path, for_each_doc_path, for_each_node_path, load_meta, load_meta_required, path_doc,
    path_node,
};

pub(super) fn validate_structure_append(
    write_txn: &WriteTransaction,
    op: &StructureOp,
    repo_scope: &str,
) -> Result<()> {
    validate_structure_state(write_txn, op)
        .map_err(|err| reject_invalid_structure(op, &err.to_string(), repo_scope))
}

fn validate_structure_state(write_txn: &WriteTransaction, op: &StructureOp) -> Result<()> {
    match op {
        StructureOp::CreateFile {
            node_id,
            doc_id,
            parent_id,
            name,
        } => {
            ensure_name_segment(name)?;
            if *node_id != NodeId::from_doc_id(*doc_id) {
                anyhow::bail!("CreateFile node/doc mismatch for {}", doc_id);
            }
            ensure_node_absent(write_txn, *node_id)?;
            ensure_parent_dir(write_txn, *parent_id)?;
            ensure_path_free(write_txn, &child_path(write_txn, *parent_id, name)?)
        }
        StructureOp::CreateDir {
            node_id,
            parent_id,
            name,
        } => {
            ensure_name_segment(name)?;
            ensure_node_absent(write_txn, *node_id)?;
            ensure_parent_dir(write_txn, *parent_id)?;
            ensure_path_free(write_txn, &child_path(write_txn, *parent_id, name)?)
        }
        StructureOp::RenameNode {
            node_id,
            doc_id,
            new_name,
        } => {
            ensure_name_segment(new_name)?;
            let meta = load_meta_required(write_txn, *node_id)?;
            ensure_doc_match(*node_id, meta.doc_id, *doc_id)?;
            let new_path = child_path(write_txn, meta.parent_id, new_name)?;
            ensure_rename_target_free(write_txn, &meta, new_path)
        }
        StructureOp::MoveNode {
            node_id,
            doc_id,
            new_parent_id,
        } => {
            let meta = load_meta_required(write_txn, *node_id)?;
            ensure_doc_match(*node_id, meta.doc_id, *doc_id)?;
            ensure_parent_dir(write_txn, *new_parent_id)?;
            ensure_not_descendant(write_txn, *node_id, *new_parent_id)?;
            let new_path = child_path(write_txn, *new_parent_id, &meta.name)?;
            ensure_rename_target_free(write_txn, &meta, new_path)
        }
        StructureOp::DeleteNode { node_id, doc_id } => {
            if let Some(meta) = load_meta(write_txn, *node_id)? {
                ensure_doc_match(*node_id, meta.doc_id, *doc_id)?;
            }
            Ok(())
        }
    }
}

fn ensure_parent_dir(write_txn: &WriteTransaction, parent_id: Option<NodeId>) -> Result<()> {
    let Some(parent_id) = parent_id else {
        return Ok(());
    };
    let parent = load_meta_required(write_txn, parent_id)?;
    if parent.kind != NodeKind::Dir {
        anyhow::bail!("structure parent is not a directory: {}", parent_id);
    }
    Ok(())
}

fn ensure_node_absent(write_txn: &WriteTransaction, node_id: NodeId) -> Result<()> {
    if load_meta(write_txn, node_id)?.is_some() {
        anyhow::bail!("structure node already exists: {}", node_id);
    }
    Ok(())
}

fn ensure_path_free(write_txn: &WriteTransaction, path: &str) -> Result<()> {
    if path_node(write_txn, path)?.is_some() {
        anyhow::bail!("structure path already bound to a node: {}", path);
    }
    if path_doc(write_txn, path)?.is_some() {
        anyhow::bail!("structure path already bound to a document: {}", path);
    }
    Ok(())
}

fn ensure_rename_target_free(
    write_txn: &WriteTransaction,
    meta: &NodeMeta,
    new_path: String,
) -> Result<()> {
    if meta.path == new_path {
        return Ok(());
    }
    if meta.kind == NodeKind::File {
        ensure_exact_target_free(write_txn, meta, &new_path)
    } else {
        ensure_prefix_target_free(write_txn, &meta.path, &new_path)
    }
}

fn ensure_exact_target_free(
    write_txn: &WriteTransaction,
    meta: &NodeMeta,
    new_path: &str,
) -> Result<()> {
    if let Some(node_id) = path_node(write_txn, new_path)? {
        anyhow::bail!("structure target path already bound to node {}", node_id);
    }
    if let Some(doc_id) = path_doc(write_txn, new_path)?
        && Some(doc_id) != meta.doc_id
    {
        anyhow::bail!("structure target path already bound to document {}", doc_id);
    }
    Ok(())
}

fn ensure_prefix_target_free(
    write_txn: &WriteTransaction,
    old_prefix: &str,
    new_prefix: &str,
) -> Result<()> {
    let old_child_prefix = child_prefix(old_prefix);
    let new_child_prefix = child_prefix(new_prefix);
    ensure_node_prefix_target_free(
        write_txn,
        old_prefix,
        &old_child_prefix,
        new_prefix,
        &new_child_prefix,
    )?;
    ensure_doc_prefix_target_free(
        write_txn,
        old_prefix,
        &old_child_prefix,
        new_prefix,
        &new_child_prefix,
    )
}

fn ensure_node_prefix_target_free(
    write_txn: &WriteTransaction,
    old_prefix: &str,
    old_child_prefix: &str,
    new_prefix: &str,
    new_child_prefix: &str,
) -> Result<()> {
    for_each_node_path(write_txn, |path| {
        if is_under(path, new_prefix, new_child_prefix)
            && !is_under(path, old_prefix, old_child_prefix)
        {
            anyhow::bail!("structure target path already bound to node: {}", path);
        }
        Ok(())
    })
}

fn ensure_doc_prefix_target_free(
    write_txn: &WriteTransaction,
    old_prefix: &str,
    old_child_prefix: &str,
    new_prefix: &str,
    new_child_prefix: &str,
) -> Result<()> {
    for_each_doc_path(write_txn, |path| {
        if is_under(path, new_prefix, new_child_prefix)
            && !is_under(path, old_prefix, old_child_prefix)
        {
            anyhow::bail!("structure target path already bound to document: {}", path);
        }
        Ok(())
    })
}

fn child_prefix(prefix: &str) -> String {
    let mut child_prefix = String::with_capacity(prefix.len() + 1);
    child_prefix.push_str(prefix);
    child_prefix.push('/');
    child_prefix
}

fn is_under(path: &str, prefix: &str, child_prefix: &str) -> bool {
    path == prefix || path.starts_with(child_prefix)
}

fn ensure_not_descendant(
    write_txn: &WriteTransaction,
    node_id: NodeId,
    new_parent_id: Option<NodeId>,
) -> Result<()> {
    let mut cursor = new_parent_id;
    let mut visiting = HashSet::new();
    while let Some(parent_id) = cursor {
        if parent_id == node_id {
            anyhow::bail!("structure move would create cycle at node {}", node_id);
        }
        if !visiting.insert(parent_id) {
            anyhow::bail!("structure projection contains cycle at node {}", parent_id);
        }
        cursor = load_meta_required(write_txn, parent_id)?.parent_id;
    }
    Ok(())
}

fn ensure_doc_match(node_id: NodeId, actual: Option<DocId>, expected: Option<DocId>) -> Result<()> {
    if actual != expected {
        anyhow::bail!(
            "structure doc mismatch for {}: actual={:?}, expected={:?}",
            node_id,
            actual,
            expected
        );
    }
    Ok(())
}

fn ensure_name_segment(name: &str) -> Result<()> {
    if name.is_empty() || name.contains('/') || name.contains('\\') {
        anyhow::bail!("invalid structure name segment: {}", name);
    }
    Ok(())
}
