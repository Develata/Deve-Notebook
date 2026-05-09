//! plan_ref:
//!   - 04_storage#projection-contract
//!   - 06_repository#tree-projection-contract
//!
use crate::models::{DocId, NodeId, NodeMeta};
use anyhow::{Result, anyhow};
use redb::{ReadableTable, WriteTransaction};

use crate::ledger::schema::{NODEID_TO_META, PATH_TO_DOCID, PATH_TO_NODEID};

pub(super) fn child_path(
    write_txn: &WriteTransaction,
    parent_id: Option<NodeId>,
    name: &str,
) -> Result<String> {
    if let Some(parent_id) = parent_id {
        let parent = load_meta_required(write_txn, parent_id)?;
        return Ok(format!("{}/{}", parent.path, name));
    }
    Ok(name.to_string())
}

pub(super) fn load_meta(write_txn: &WriteTransaction, node_id: NodeId) -> Result<Option<NodeMeta>> {
    let table = write_txn.open_table(NODEID_TO_META)?;
    table
        .get(node_id.as_u128())?
        .map(|bytes| bincode::deserialize(bytes.value()).map_err(Into::into))
        .transpose()
}

pub(super) fn load_meta_required(
    write_txn: &WriteTransaction,
    node_id: NodeId,
) -> Result<NodeMeta> {
    load_meta(write_txn, node_id)?
        .ok_or_else(|| anyhow!("structure projection missing node {}", node_id))
}

pub(super) fn path_node(write_txn: &WriteTransaction, path: &str) -> Result<Option<NodeId>> {
    let table = write_txn.open_table(PATH_TO_NODEID)?;
    Ok(table.get(path)?.map(|v| NodeId::from_u128(v.value())))
}

pub(super) fn path_doc(write_txn: &WriteTransaction, path: &str) -> Result<Option<DocId>> {
    let table = write_txn.open_table(PATH_TO_DOCID)?;
    Ok(table.get(path)?.map(|v| DocId::from_u128(v.value())))
}

pub(super) fn for_each_node_path(
    write_txn: &WriteTransaction,
    mut visit: impl FnMut(&str) -> Result<()>,
) -> Result<()> {
    let table = write_txn.open_table(PATH_TO_NODEID)?;
    for item in table.iter()? {
        let (path_guard, _) = item?;
        visit(path_guard.value())?;
    }
    Ok(())
}

pub(super) fn for_each_doc_path(
    write_txn: &WriteTransaction,
    mut visit: impl FnMut(&str) -> Result<()>,
) -> Result<()> {
    let table = write_txn.open_table(PATH_TO_DOCID)?;
    for item in table.iter()? {
        let (path_guard, _) = item?;
        visit(path_guard.value())?;
    }
    Ok(())
}
