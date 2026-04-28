// crates/core/src/ledger/node_meta/migrate.rs
//! plan_ref:
//!   - 06_repository#tree-projection-contract
//!   - 04_storage#projection-contract
//!
//! # Node 元数据迁移/查询

use crate::ledger::schema::{INODE_TO_NODEID, NODEID_TO_META, PATH_TO_NODEID};
use crate::models::{DocId, NodeId, NodeKind, NodeMeta};
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

pub fn list_file_docs(db: &Database) -> Result<Vec<(DocId, String)>> {
    let mut docs = Vec::new();
    for (_, meta) in list_nodes(db)? {
        if meta.kind == NodeKind::File
            && let Some(doc_id) = meta.doc_id
        {
            docs.push((doc_id, meta.path));
        }
    }
    docs.sort_by(|left, right| left.1.cmp(&right.1));
    Ok(docs)
}

pub fn file_meta_for_doc(db: &Database, doc_id: DocId) -> Result<Option<NodeMeta>> {
    ensure_node_tables(db)?;
    let read_txn = db.begin_read()?;
    let table = read_txn.open_table(NODEID_TO_META)?;
    let Some(meta_bytes) = table.get(NodeId::from_doc_id(doc_id).as_u128())? else {
        return Ok(None);
    };
    let meta: NodeMeta = bincode::deserialize(meta_bytes.value())?;
    if meta.kind != NodeKind::File || meta.doc_id != Some(doc_id) {
        return Ok(None);
    }
    Ok(Some(meta))
}

pub fn path_for_doc(db: &Database, doc_id: DocId) -> Result<Option<String>> {
    Ok(file_meta_for_doc(db, doc_id)?.map(|meta| meta.path))
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
