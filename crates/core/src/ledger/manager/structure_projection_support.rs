//! plan_ref:
//!   - 06_repository#tree-projection-contract

use crate::ledger::node_meta;
use crate::models::{DocId, NodeId, NodeMeta};
use anyhow::{Result, anyhow};
use redb::WriteTransaction;

pub(super) fn child_path_in_txn(
    write_txn: &WriteTransaction,
    parent_id: Option<NodeId>,
    name: &str,
) -> Result<String> {
    if let Some(parent_id) = parent_id {
        let parent = load_meta_in_txn(write_txn, parent_id, None)?;
        return Ok(format!("{}/{}", parent.path, name));
    }
    Ok(name.to_string())
}

pub(super) fn load_meta_in_txn(
    write_txn: &WriteTransaction,
    node_id: NodeId,
    doc_id: Option<DocId>,
) -> Result<NodeMeta> {
    let meta = node_meta::get_node_meta_in_txn(write_txn, node_id)?
        .ok_or_else(|| anyhow!("node meta missing for {}", node_id))?;
    ensure_doc_match(node_id, meta.doc_id, doc_id)?;
    Ok(meta)
}

pub(super) fn ensure_doc_match(
    node_id: NodeId,
    actual: Option<DocId>,
    expected: Option<DocId>,
) -> Result<()> {
    if actual != expected {
        return Err(anyhow!(
            "structure doc mismatch for {}: actual={:?}, expected={:?}",
            node_id,
            actual,
            expected
        ));
    }
    Ok(())
}
