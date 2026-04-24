//! plan_ref:
//!   - 06_repository#tree-projection-contract

use crate::ledger::{metadata, node_meta};
use crate::models::{DocId, NodeId, NodeKind, NodeMeta, StructureOp};
use anyhow::{Result, anyhow};
use redb::Database;

use super::structure_projection_support::{child_path, ensure_doc_match, load_meta};

/// Invariants:
/// - 这里只做 Structure Facts -> projection 的受控折叠。
/// - `metadata` 直写只能出现在本模块这类 projection internals。
pub(crate) fn apply(db: &Database, op: &StructureOp) -> Result<()> {
    match op {
        StructureOp::CreateFile {
            node_id,
            doc_id,
            parent_id,
            name,
        } => apply_create_file(db, *node_id, *doc_id, *parent_id, name),
        StructureOp::CreateDir {
            node_id,
            parent_id,
            name,
        } => apply_create_dir(db, *node_id, *parent_id, name),
        StructureOp::RenameNode {
            node_id,
            doc_id,
            new_name,
        } => {
            let meta = load_meta(db, *node_id, *doc_id)?;
            rename_path(db, &meta, child_path(db, meta.parent_id, new_name)?)
        }
        StructureOp::MoveNode {
            node_id,
            doc_id,
            new_parent_id,
        } => {
            let meta = load_meta(db, *node_id, *doc_id)?;
            rename_path(db, &meta, child_path(db, *new_parent_id, &meta.name)?)
        }
        StructureOp::DeleteNode { node_id, doc_id } => delete_node(db, *node_id, *doc_id),
    }
}

fn apply_create_file(
    db: &Database,
    node_id: NodeId,
    doc_id: DocId,
    parent_id: Option<NodeId>,
    name: &str,
) -> Result<()> {
    if node_id != NodeId::from_doc_id(doc_id) {
        return Err(anyhow!("CreateFile node/doc mismatch for {}", doc_id));
    }
    metadata::set_doc_path(db, doc_id, &child_path(db, parent_id, name)?)
}

fn apply_create_dir(
    db: &Database,
    node_id: NodeId,
    parent_id: Option<NodeId>,
    name: &str,
) -> Result<()> {
    let meta = NodeMeta {
        kind: NodeKind::Dir,
        name: name.to_string(),
        parent_id,
        path: child_path(db, parent_id, name)?,
        doc_id: None,
    };
    node_meta::upsert_node(db, node_id, &meta)
}

fn delete_node(db: &Database, node_id: NodeId, doc_id: Option<DocId>) -> Result<()> {
    let Some(meta) = node_meta::get_node_meta(db, node_id)? else {
        return Ok(());
    };
    ensure_doc_match(node_id, meta.doc_id, doc_id)?;
    if meta.doc_id.is_some() {
        metadata::delete_doc(db, &meta.path)
    } else {
        metadata::delete_folder(db, &meta.path).map(|_| ())
    }
}

fn rename_path(db: &Database, meta: &NodeMeta, new_path: String) -> Result<()> {
    if meta.path == new_path {
        return Ok(());
    }
    match meta.doc_id {
        Some(_) => metadata::rename_doc(db, &meta.path, &new_path),
        None => node_meta::rename_path_prefix(db, &meta.path, &new_path),
    }
}
