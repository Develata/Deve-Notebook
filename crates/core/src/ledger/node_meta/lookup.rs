//! plan_ref:
//!   - 06_repository#tree-projection-contract
//!   - 04_storage#projection-contract
//!   - 04_storage#internal-path-normalization

use crate::ledger::schema::{NODEID_TO_META, PATH_TO_DOCID, PATH_TO_NODEID};
use crate::models::{DocId, NodeId, NodeMeta};
use crate::utils::path::to_forward_slash;
use anyhow::Result;
use redb::{Database, ReadableTable, WriteTransaction};

pub fn get_node_id(db: &Database, path: &str) -> Result<Option<NodeId>> {
    let normalized = to_forward_slash(path);
    let read_txn = db.begin_read()?;
    let table = read_txn.open_table(PATH_TO_NODEID)?;
    if let Some(v) = table.get(&*normalized)? {
        Ok(Some(NodeId::from_u128(v.value())))
    } else {
        Ok(None)
    }
}

pub(crate) fn get_node_id_in_txn(
    write_txn: &WriteTransaction,
    path: &str,
) -> Result<Option<NodeId>> {
    let normalized = to_forward_slash(path);
    let table = write_txn.open_table(PATH_TO_NODEID)?;
    Ok(table
        .get(&*normalized)?
        .map(|v| NodeId::from_u128(v.value())))
}

pub fn get_node_meta(db: &Database, node_id: NodeId) -> Result<Option<NodeMeta>> {
    let read_txn = db.begin_read()?;
    let table = read_txn.open_table(NODEID_TO_META)?;
    if let Some(v) = table.get(node_id.as_u128())? {
        let meta: NodeMeta = bincode::deserialize(v.value())?;
        Ok(Some(meta))
    } else {
        Ok(None)
    }
}

pub(crate) fn get_node_meta_in_txn(
    write_txn: &WriteTransaction,
    node_id: NodeId,
) -> Result<Option<NodeMeta>> {
    let table = write_txn.open_table(NODEID_TO_META)?;
    table
        .get(node_id.as_u128())?
        .map(|v| bincode::deserialize(v.value()).map_err(Into::into))
        .transpose()
}

pub(super) fn path_doc_in_txn(write_txn: &WriteTransaction, path: &str) -> Result<Option<DocId>> {
    let table = write_txn.open_table(PATH_TO_DOCID)?;
    Ok(table.get(path)?.map(|v| DocId::from_u128(v.value())))
}
