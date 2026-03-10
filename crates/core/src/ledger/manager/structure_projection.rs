use crate::ledger::{metadata, node_meta, ops};
use crate::models::{DocId, NodeId, NodeKind, NodeMeta, StructureOp};
use anyhow::{Result, anyhow};
use redb::Database;

#[cfg(test)]
#[path = "projection_cleanup_test.rs"]
mod tests;

/// Invariants:
/// - 这里只做 Structure Facts -> projection 的受控折叠。
/// - `metadata` 直写只能出现在本模块这类 projection internals。
pub(super) fn apply(db: &Database, op: &StructureOp) -> Result<()> {
    match op {
        StructureOp::CreateFile {
            node_id,
            parent_id,
            name,
        } => apply_create_file(db, *node_id, *parent_id, name),
        StructureOp::CreateDir {
            node_id,
            parent_id,
            name,
        } => apply_create_dir(db, *node_id, *parent_id, name),
        StructureOp::RenameNode { node_id, new_name } => {
            let meta = load_meta(db, *node_id)?;
            rename_path(db, &meta, child_path(db, meta.parent_id, new_name)?)
        }
        StructureOp::MoveNode {
            node_id,
            new_parent_id,
        } => {
            let meta = load_meta(db, *node_id)?;
            rename_path(db, &meta, child_path(db, *new_parent_id, &meta.name)?)
        }
        StructureOp::DeleteNode { node_id } => delete_node(db, *node_id),
    }
}

/// Pre-conditions:
/// - `path` 已规范化为 forward-slash。
///
/// Post-conditions:
/// - 仅当该路径对应实体在 Ledger 中完全没有事实时，才移除孤立 projection。
///
/// Invariants:
/// - 业务路径不得直接删 metadata；只能调用 projection helper 做孤儿清理。
pub(super) fn drop_transient_file_path(db: &Database, path: &str) -> Result<()> {
    let Some(doc_id) = metadata::get_docid(db, path)? else {
        return Ok(());
    };
    if ops::count_ops_from_db(db, doc_id)? > 0 {
        return Ok(());
    }
    metadata::delete_doc(db, path)
}

fn apply_create_file(
    db: &Database,
    node_id: NodeId,
    parent_id: Option<NodeId>,
    name: &str,
) -> Result<()> {
    let doc_id = DocId::from_u128(node_id.as_u128());
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

fn delete_node(db: &Database, node_id: NodeId) -> Result<()> {
    let Some(meta) = node_meta::get_node_meta(db, node_id)? else {
        return Ok(());
    };
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
        Some(doc_id) => metadata::set_doc_path(db, doc_id, &new_path),
        None => node_meta::rename_path_prefix(db, &meta.path, &new_path),
    }
}

fn child_path(db: &Database, parent_id: Option<NodeId>, name: &str) -> Result<String> {
    if let Some(parent_id) = parent_id {
        let parent = load_meta(db, parent_id)?;
        return Ok(format!("{}/{}", parent.path, name));
    }
    Ok(name.to_string())
}

fn load_meta(db: &Database, node_id: NodeId) -> Result<NodeMeta> {
    node_meta::get_node_meta(db, node_id)?
        .ok_or_else(|| anyhow!("node meta missing for {}", node_id))
}
