//! plan_ref:
//!   - 03_storage#repo-runtime-layout
//!   - 05_diff_logic#source-control-runtime

use crate::ledger::schema::STAGED_DOC_INDEX;
use crate::models::DocId;
use anyhow::Result;
use redb::{Database, WriteTransaction};

pub(super) fn init_table(write_txn: &WriteTransaction) -> Result<()> {
    let _ = write_txn.open_multimap_table(STAGED_DOC_INDEX)?;
    Ok(())
}

pub(super) fn replace(
    write_txn: &WriteTransaction,
    old: Option<DocId>,
    new: Option<DocId>,
    path: &str,
) -> Result<()> {
    let mut table = write_txn.open_multimap_table(STAGED_DOC_INDEX)?;
    if let Some(doc_id) = old {
        let _ = table.remove(doc_id.as_u128(), path)?;
    }
    if let Some(doc_id) = new {
        let _ = table.insert(doc_id.as_u128(), path)?;
    }
    Ok(())
}

pub(super) fn remove(
    write_txn: &WriteTransaction,
    doc_id: Option<DocId>,
    path: &str,
) -> Result<()> {
    if let Some(doc_id) = doc_id {
        let mut table = write_txn.open_multimap_table(STAGED_DOC_INDEX)?;
        let _ = table.remove(doc_id.as_u128(), path)?;
    }
    Ok(())
}

pub fn paths_for_doc(db: &Database, doc_id: DocId) -> Result<Vec<String>> {
    let read_txn = db.begin_read()?;
    let table = read_txn.open_multimap_table(STAGED_DOC_INDEX)?;
    let mut paths = Vec::new();
    for path in table.get(doc_id.as_u128())? {
        paths.push(path?.value().to_string());
    }
    paths.sort();
    paths.dedup();
    Ok(paths)
}
