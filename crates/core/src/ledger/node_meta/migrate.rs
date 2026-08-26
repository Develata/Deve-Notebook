// crates/core/src/ledger/node_meta/migrate.rs
//! plan_ref:
//!   - 04_repository#tree-projection-contract
//!   - 03_storage/projection#projection-contract
//!
//! # Node 元数据迁移/查询

use crate::codec;
use crate::ledger::schema::{INODE_TO_NODEID, NODEID_TO_META, PATH_TO_NODEID};
use crate::models::{DocId, NodeId, NodeKind, NodeMeta};
use anyhow::{Result, anyhow};
use redb::{Database, ReadableTable};

pub fn list_nodes(db: &Database) -> Result<Vec<(NodeId, NodeMeta)>> {
    ensure_node_tables(db)?;
    let read_txn = db.begin_read()?;
    let table = read_txn.open_table(NODEID_TO_META)?;
    let mut nodes = Vec::new();
    for item in table.iter()? {
        let (id, meta_bytes) = item?;
        let meta: NodeMeta = codec::decode(meta_bytes.value())?;
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

pub fn count_file_docs(db: &Database) -> Result<u64> {
    let read_txn = db.begin_read()?;
    let table = read_txn.open_table(NODEID_TO_META)?;
    let mut count = 0u64;
    for item in table.iter()? {
        let (_, meta_bytes) = item?;
        let meta: NodeMeta = codec::decode(meta_bytes.value())?;
        if meta.kind == NodeKind::File && meta.doc_id.is_some() {
            count = count
                .checked_add(1)
                .ok_or_else(|| anyhow!("local document count overflow"))?;
        }
    }
    Ok(count)
}

pub fn file_meta_for_doc(db: &Database, doc_id: DocId) -> Result<Option<NodeMeta>> {
    ensure_node_tables(db)?;
    let read_txn = db.begin_read()?;
    let table = read_txn.open_table(NODEID_TO_META)?;
    let Some(meta_bytes) = table.get(NodeId::from_doc_id(doc_id).as_u128())? else {
        return Ok(None);
    };
    let meta: NodeMeta = codec::decode(meta_bytes.value())?;
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

#[cfg(test)]
mod tests {
    use super::count_file_docs;
    use crate::ledger::node_meta::{create_dir_node, ensure_file_node};
    use crate::models::DocId;

    #[test]
    fn count_file_docs_streams_file_rows_without_materializing_paths() -> anyhow::Result<()> {
        let dir = tempfile::tempdir()?;
        let db = redb::Database::create(dir.path().join("node-meta.redb"))?;
        super::ensure_node_tables(&db)?;
        create_dir_node(&db, "notes")?;
        ensure_file_node(&db, "notes/a.md", DocId::new())?;
        ensure_file_node(&db, "notes/b.md", DocId::new())?;

        assert_eq!(count_file_docs(&db)?, 2);
        Ok(())
    }
}
