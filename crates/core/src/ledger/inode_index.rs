//! # Watcher Identity Index
//!
//! `inode -> doc_id` 只服务于 watcher 的稳定身份识别，
//! 不是 projection 真值，也不属于 path/doc metadata 语义。

use crate::ledger::schema::INODE_TO_DOCID;
use crate::models::{DocId, FileNodeId};
use anyhow::Result;
use redb::Database;

pub fn get_docid(db: &Database, inode: &FileNodeId) -> Result<Option<DocId>> {
    let read_txn = db.begin_read()?;
    let table = read_txn.open_table(INODE_TO_DOCID)?;
    if let Some(v) = table.get(inode.id)? {
        Ok(Some(DocId::from_u128(v.value())))
    } else {
        Ok(None)
    }
}

pub fn bind_docid(db: &Database, inode: &FileNodeId, doc_id: DocId) -> Result<()> {
    let write_txn = db.begin_write()?;
    {
        let mut table = write_txn.open_table(INODE_TO_DOCID)?;
        table.insert(inode.id, doc_id.as_u128())?;
    }
    write_txn.commit()?;
    Ok(())
}
