use super::{STAGED_TABLE, StagedEntry, index};
use crate::models::DocId;
use crate::source_control::ChangeStatus;
use anyhow::Result;
use redb::{Database, ReadableTable};

pub fn get_staged(db: &Database, path: &str) -> Result<Option<StagedEntry>> {
    let read_txn = db.begin_read()?;
    let table = read_txn.open_table(STAGED_TABLE)?;
    table
        .get(path)?
        .map(|guard| serde_json::from_slice(guard.value()))
        .transpose()
        .map_err(Into::into)
}

pub fn list_staged(db: &Database) -> Result<Vec<String>> {
    Ok(list_staged_entries(db)?
        .into_iter()
        .map(|(path, _)| path)
        .collect())
}

pub fn list_staged_with_status(db: &Database) -> Result<Vec<(String, ChangeStatus)>> {
    Ok(list_staged_entries(db)?
        .into_iter()
        .map(|(path, entry)| (path, entry.status))
        .collect())
}

pub fn list_staged_entries(db: &Database) -> Result<Vec<(String, StagedEntry)>> {
    let read_txn = db.begin_read()?;
    let table = read_txn.open_table(STAGED_TABLE)?;
    let mut entries = Vec::new();
    for item in table.iter()? {
        let (key, value) = item?;
        entries.push((
            key.value().to_string(),
            serde_json::from_slice(value.value())?,
        ));
    }
    Ok(entries)
}

pub fn list_staged_entries_for_doc(
    db: &Database,
    doc_id: DocId,
) -> Result<Vec<(String, StagedEntry)>> {
    let mut entries = Vec::new();
    let paths = index::paths_for_doc(db, doc_id)?;
    if paths.is_empty() {
        for (path, entry) in list_staged_entries(db)? {
            if entry.doc_id == Some(doc_id) {
                entries.push((path, entry));
            }
        }
        return Ok(entries);
    }
    for path in paths {
        if let Some(entry) = get_staged(db, &path)?
            && entry.doc_id == Some(doc_id)
        {
            entries.push((path, entry));
        }
    }
    Ok(entries)
}

pub fn is_staged(db: &Database, path: &str) -> Result<bool> {
    Ok(get_staged(db, path)?.is_some())
}
