//! plan_ref:
//!   - 06_repository#tree-projection-contract

use crate::ledger::{metadata, node_meta};
use crate::models::{DocId, NodeId, NodeKind, NodeMeta, StructureOp};
use anyhow::{Result, anyhow};
use redb::{Database, WriteTransaction};

use super::structure_projection_support::{child_path_in_txn, ensure_doc_match, load_meta_in_txn};

/// Invariants:
/// - 这里只做 Structure Facts -> projection 的受控折叠。
/// - `metadata` 直写只能出现在本模块这类 projection internals。
pub(crate) fn apply(db: &Database, op: &StructureOp) -> Result<()> {
    let write_txn = db.begin_write()?;
    apply_in_txn(&write_txn, op)?;
    write_txn.commit()?;
    Ok(())
}

pub(crate) fn apply_in_txn(write_txn: &WriteTransaction, op: &StructureOp) -> Result<()> {
    match op {
        StructureOp::CreateFile {
            node_id,
            doc_id,
            parent_id,
            name,
        } => apply_create_file(write_txn, *node_id, *doc_id, *parent_id, name),
        StructureOp::CreateDir {
            node_id,
            parent_id,
            name,
        } => apply_create_dir(write_txn, *node_id, *parent_id, name),
        StructureOp::RenameNode {
            node_id,
            doc_id,
            new_name,
        } => {
            let meta = load_meta_in_txn(write_txn, *node_id, *doc_id)?;
            let new_path = child_path_in_txn(write_txn, meta.parent_id, new_name)?;
            rename_path(write_txn, &meta, new_path)
        }
        StructureOp::MoveNode {
            node_id,
            doc_id,
            new_parent_id,
        } => {
            let meta = load_meta_in_txn(write_txn, *node_id, *doc_id)?;
            let new_path = child_path_in_txn(write_txn, *new_parent_id, &meta.name)?;
            rename_path(write_txn, &meta, new_path)
        }
        StructureOp::DeleteNode { node_id, doc_id } => delete_node(write_txn, *node_id, *doc_id),
    }
}

fn apply_create_file(
    write_txn: &WriteTransaction,
    node_id: NodeId,
    doc_id: DocId,
    parent_id: Option<NodeId>,
    name: &str,
) -> Result<()> {
    if node_id != NodeId::from_doc_id(doc_id) {
        return Err(anyhow!("CreateFile node/doc mismatch for {}", doc_id));
    }
    let path = child_path_in_txn(write_txn, parent_id, name)?;
    metadata::set_doc_path_in_txn(write_txn, doc_id, &path)
}

fn apply_create_dir(
    write_txn: &WriteTransaction,
    node_id: NodeId,
    parent_id: Option<NodeId>,
    name: &str,
) -> Result<()> {
    let meta = NodeMeta {
        kind: NodeKind::Dir,
        name: name.to_string(),
        parent_id,
        path: child_path_in_txn(write_txn, parent_id, name)?,
        doc_id: None,
    };
    node_meta::upsert_node_in_txn(write_txn, node_id, &meta)
}

fn delete_node(write_txn: &WriteTransaction, node_id: NodeId, doc_id: Option<DocId>) -> Result<()> {
    let Some(meta) = node_meta::get_node_meta_in_txn(write_txn, node_id)? else {
        return Ok(());
    };
    ensure_doc_match(node_id, meta.doc_id, doc_id)?;
    if meta.doc_id.is_some() {
        metadata::delete_doc_in_txn(write_txn, &meta.path)
    } else {
        metadata::delete_folder_in_txn(write_txn, &meta.path).map(|_| ())
    }
}

fn rename_path(write_txn: &WriteTransaction, meta: &NodeMeta, new_path: String) -> Result<()> {
    if meta.path == new_path {
        return Ok(());
    }
    match meta.doc_id {
        Some(_) => metadata::rename_doc_in_txn(write_txn, &meta.path, &new_path),
        None => metadata::rename_folder_in_txn(write_txn, &meta.path, &new_path),
    }
}
