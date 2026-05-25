//! plan_ref:
//!   - 03_storage/index#repo-runtime-layout
//!   - 03_storage/watcher#watcher-contract

use super::{PendingFsEntry, index};
use crate::ledger::schema::PENDING_FS_OPS;
use crate::models::DocId;
use anyhow::Result;
use redb::{Database, ReadableTable};

pub fn list_all(db: &Database) -> Result<Vec<PendingFsEntry>> {
    let read_txn = db.begin_read()?;
    let table = read_txn.open_table(PENDING_FS_OPS)?;
    let mut entries = Vec::new();
    for item in table.iter()? {
        let (_key, value) = item?;
        entries.push(serde_json::from_slice(value.value())?);
    }
    Ok(entries)
}

pub fn get(db: &Database, path: &str) -> Result<Option<PendingFsEntry>> {
    let read_txn = db.begin_read()?;
    let table = read_txn.open_table(PENDING_FS_OPS)?;
    table
        .get(path)?
        .map(|guard| serde_json::from_slice(guard.value()))
        .transpose()
        .map_err(Into::into)
}

pub fn list_for_doc(db: &Database, doc_id: DocId) -> Result<Vec<PendingFsEntry>> {
    let mut entries = Vec::new();
    let paths = index::paths_for_doc(db, doc_id)?;
    if paths.is_empty() {
        for entry in list_all(db)? {
            if entry.doc_id == Some(doc_id) {
                entries.push(entry);
            }
        }
        return Ok(entries);
    }
    for path in paths {
        if let Some(entry) = get(db, &path)?
            && entry.doc_id == Some(doc_id)
        {
            entries.push(entry);
        }
    }
    Ok(entries)
}
