use crate::ledger::{metadata, node_meta};
use crate::models::DocId;
use anyhow::Result;
use redb::Database;

/// Invariants:
/// - `path -> DocId` 解析优先依赖 Node projection。
/// - 仅在 Node projection 缺失时，才回退到 legacy metadata 映射。
pub fn resolve_doc_id(db: &Database, path: &str) -> Result<Option<DocId>> {
    if let Some(node_id) = node_meta::get_node_id(db, path)?
        && let Some(meta) = node_meta::get_node_meta(db, node_id)?
        && meta.doc_id.is_some()
    {
        return Ok(meta.doc_id);
    }
    metadata::get_docid(db, path)
}
