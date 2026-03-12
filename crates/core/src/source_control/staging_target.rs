use super::{
    StagedEntry, get_staged, list_staged_entries, list_staged_entries_for_doc, take_staged,
};
use crate::models::DocId;
use crate::protocol::ScPathTarget;
use crate::source_control::ChangeStatus;
use crate::utils::path::to_forward_slash;
use anyhow::Result;
use redb::Database;

pub fn get_staged_for_target(
    db: &Database,
    target: &ScPathTarget,
) -> Result<Option<(String, StagedEntry)>> {
    let path = to_forward_slash(&target.path);
    if let Some(doc_id) = target.doc_id
        && let Some(entry) = resolve_for_doc(db, &path, doc_id)?
    {
        return Ok(Some(entry));
    }
    if let Some(entry) = get_staged(db, &path)? {
        return Ok(Some((path, entry)));
    }
    resolve_without_doc(db, &path)
}

pub fn take_staged_for_target(
    db: &Database,
    target: &ScPathTarget,
) -> Result<Option<(String, StagedEntry)>> {
    let Some((path, entry)) = get_staged_for_target(db, target)? else {
        return Ok(None);
    };
    let _ = take_staged(db, &path)?;
    Ok(Some((path, entry)))
}

fn resolve_for_doc(
    db: &Database,
    path: &str,
    doc_id: DocId,
) -> Result<Option<(String, StagedEntry)>> {
    Ok(select_entry(
        list_staged_entries_for_doc(db, doc_id)?,
        path,
        Some(doc_id),
    ))
}

fn resolve_without_doc(db: &Database, path: &str) -> Result<Option<(String, StagedEntry)>> {
    Ok(select_entry(list_staged_entries(db)?, path, None))
}

fn select_entry(
    entries: Vec<(String, StagedEntry)>,
    path: &str,
    doc_id: Option<DocId>,
) -> Option<(String, StagedEntry)> {
    let exact = entries
        .iter()
        .find(|(entry_path, entry)| {
            entry_path == path && doc_id.map(|id| entry.doc_id == Some(id)).unwrap_or(true)
        })
        .cloned();
    if exact.is_some() {
        return exact;
    }
    entries
        .iter()
        .find(|(_, entry)| {
            entry.doc_id == doc_id
                && entry.status != ChangeStatus::Deleted
                && entry.renamed_from.as_deref().map(to_forward_slash) == Some(path.to_string())
        })
        .cloned()
        .or_else(|| {
            entries
                .iter()
                .find(|(_, entry)| entry.doc_id == doc_id && entry.status != ChangeStatus::Deleted)
                .cloned()
        })
        .or_else(|| {
            entries
                .iter()
                .find(|(entry_path, entry)| {
                    entry_path == path && entry.status != ChangeStatus::Deleted
                })
                .cloned()
        })
        .or_else(|| {
            entries
                .into_iter()
                .find(|(entry_path, _)| entry_path == path)
        })
}
