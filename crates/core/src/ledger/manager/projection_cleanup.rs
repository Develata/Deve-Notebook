use crate::ledger::{metadata, node_meta, ops};
use crate::source_control::{pending_fs, staging};
use anyhow::Result;
use redb::Database;

#[cfg(test)]
#[path = "projection_cleanup_test.rs"]
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
    if let Some(node_id) = node_meta::get_node_id(db, path)?
        && let Some(meta) = node_meta::get_node_meta(db, node_id)?
    {
        let has_structure = !ops::get_structure_ops_for_node_from_db(db, node_id)?.is_empty();
        let has_content = match meta.doc_id {
            Some(doc_id) => ops::count_ops_from_db(db, doc_id)? > 0,
            None => false,
        };
        if has_structure || has_content {
            return Ok(());
        }
        return if meta.doc_id.is_some() {
            metadata::delete_doc(db, path)
        } else {
            metadata::delete_folder(db, path).map(|_| ())
        };
    }
    if metadata::get_docid(db, path)?.is_some() {
        metadata::delete_doc(db, path)?;
    }
    Ok(())
}
