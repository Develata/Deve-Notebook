// crates/core/src/ledger/node_meta/migrate.rs
//! # Node 元数据迁移/查询

use super::core::ensure_file_node;
use crate::ledger::schema::{INODE_TO_NODEID, NODEID_TO_META, PATH_TO_NODEID};
use crate::ledger::{metadata, node_check};
use crate::models::{NodeId, NodeMeta};
use anyhow::Result;
use redb::{Database, ReadableTable};

pub fn list_nodes(db: &Database) -> Result<Vec<(NodeId, NodeMeta)>> {
    ensure_node_tables(db)?;
    let read_txn = db.begin_read()?;
    let table = read_txn.open_table(NODEID_TO_META)?;
    let mut nodes = Vec::new();
    for item in table.iter()? {
        let (id, meta_bytes) = item?;
        let meta: NodeMeta = bincode::deserialize(meta_bytes.value())?;
        nodes.push((NodeId::from_u128(id.value()), meta));
    }
    Ok(nodes)
}

pub fn migrate_nodes_from_docs(db: &Database) -> Result<()> {
    ensure_node_tables(db)?;
    let read_txn = db.begin_read()?;
    if let Ok(table) = read_txn.open_table(NODEID_TO_META)
        && table.iter()?.next().is_some()
    {
        return Ok(());
    }
    drop(read_txn);

    let docs = metadata::list_docs(db)?;
    for (doc_id, path) in docs {
        ensure_file_node(db, &path, doc_id)?;
    }

    let _ = node_check::repair_missing_nodes(db)?;
    Ok(())
}

fn ensure_node_tables(db: &Database) -> Result<()> {
    let write_txn = db.begin_write()?;
    {
        let _ = write_txn.open_table(NODEID_TO_META)?;
        let _ = write_txn.open_table(PATH_TO_NODEID)?;
        let _ = write_txn.open_table(INODE_TO_NODEID)?;
    }
    write_txn.commit()?;
    Ok(())
}
