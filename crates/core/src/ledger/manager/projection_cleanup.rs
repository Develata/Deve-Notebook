//! plan_ref:
//!   - 04_repository#tree-projection-contract

use crate::ledger::{metadata, node_meta, ops};
use crate::models::NodeId;
use crate::source_control::{pending_fs, staging};
use anyhow::Result;
use redb::Database;

#[cfg(test)]
mod tests;

/// Post-conditions:
/// - 仅当该路径既无 ledger facts、也无 pending/staged 锚点时，才移除孤立 projection。
///
/// Invariants:
/// - 这是 projection cleanup helper，不得用于改变权威业务状态。
pub(super) fn drop_unanchored_projection_path(db: &Database, path: &str) -> Result<()> {
    if pending_fs::get(db, path)?.is_some() || staging::get_staged(db, path)?.is_some() {
        return Ok(());
    }
    if let Some(node_id) = node_meta::get_node_id(db, path)? {
        return drop_node_projection(db, path, node_id);
    }
    if let Some(doc_id) = metadata::get_docid(db, path)? {
        return drop_doc_projection(db, path, doc_id);
    }
    metadata::delete_doc(db, path)?;
    Ok(())
}

fn drop_node_projection(db: &Database, path: &str, node_id: NodeId) -> Result<()> {
    let Some(meta) = node_meta::get_node_meta(db, node_id)? else {
        return Ok(());
    };
    if has_anchored_facts(db, node_id, meta.doc_id)? {
        return Ok(());
    }
    if meta.doc_id.is_some() {
        metadata::delete_doc(db, path)
    } else {
        metadata::delete_folder(db, path).map(|_| ())
    }
}

fn drop_doc_projection(db: &Database, path: &str, doc_id: crate::models::DocId) -> Result<()> {
    if let Some(meta) = node_meta::file_meta_for_doc(db, doc_id)? {
        if meta.path != path {
            metadata::delete_doc(db, path)?;
            return Ok(());
        }
        return drop_node_projection(db, path, NodeId::from_doc_id(doc_id));
    }
    metadata::delete_doc(db, path)?;
    Ok(())
}

fn has_anchored_facts(
    db: &Database,
    node_id: NodeId,
    doc_id: Option<crate::models::DocId>,
) -> Result<bool> {
    let has_structure = !ops::get_structure_ops_for_node_from_db(db, node_id)?.is_empty();
    let has_content = match doc_id {
        Some(doc_id) => ops::count_ops_from_db(db, doc_id)? > 0,
        None => false,
    };
    Ok(has_structure || has_content)
}
