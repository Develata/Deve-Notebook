use super::{
    StagedEntry, get_staged, list_staged_entries, list_staged_entries_for_doc, take_staged,
};
use crate::models::DocId;
use crate::protocol::ScPathTarget;
use crate::source_control::ChangeStatus;
use crate::utils::path::to_forward_slash;
use anyhow::{Result, anyhow};
use redb::Database;

pub fn get_staged_for_target(
    db: &Database,
    target: &ScPathTarget,
) -> Result<Option<(String, StagedEntry)>> {
    let path = to_forward_slash(&target.path);
    if let Some(doc_id) = target.doc_id {
        return resolve_for_doc(db, &path, doc_id);
    }
    if let Some(entry) = get_staged(db, &path)?.filter(|entry| entry.doc_id.is_none()) {
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
    select_entry(list_staged_entries_for_doc(db, doc_id)?, path, Some(doc_id))
}

fn resolve_without_doc(db: &Database, path: &str) -> Result<Option<(String, StagedEntry)>> {
    select_entry(list_staged_entries(db)?, path, None)
}

fn select_entry(
    entries: Vec<(String, StagedEntry)>,
    path: &str,
    doc_id: Option<DocId>,
) -> Result<Option<(String, StagedEntry)>> {
    if let Some(doc_id) = doc_id {
        return Ok(select_entry_for_doc(entries, path, doc_id));
    }
    select_entry_without_doc(entries, path)
}

fn select_entry_for_doc(
    entries: Vec<(String, StagedEntry)>,
    path: &str,
    doc_id: DocId,
) -> Option<(String, StagedEntry)> {
    let exact = entries
        .iter()
        .find(|(entry_path, entry)| entry_path == path && entry.doc_id == Some(doc_id))
        .cloned();
    if exact.is_some() {
        return exact;
    }
    entries
        .iter()
        .find(|(_, entry)| {
            entry.doc_id == Some(doc_id)
                && entry.status != ChangeStatus::Deleted
                && entry.renamed_from.as_deref().map(to_forward_slash) == Some(path.to_string())
        })
        .cloned()
        .or_else(|| {
            entries
                .iter()
                .find(|(_, entry)| {
                    entry.doc_id == Some(doc_id) && entry.status != ChangeStatus::Deleted
                })
                .cloned()
        })
}

fn select_entry_without_doc(
    entries: Vec<(String, StagedEntry)>,
    path: &str,
) -> Result<Option<(String, StagedEntry)>> {
    let exact = entries
        .iter()
        .find(|(entry_path, _)| entry_path == path)
        .cloned()
        .into_iter()
        .collect::<Vec<_>>();
    let renamed = entries
        .iter()
        .filter(|(_, entry)| {
            entry.status != ChangeStatus::Deleted
                && entry.renamed_from.as_deref().map(to_forward_slash) == Some(path.to_string())
        })
        .cloned()
        .collect::<Vec<_>>();
    if exact
        .iter()
        .chain(renamed.iter())
        .any(|(_, entry)| entry.doc_id.is_some())
    {
        return Err(anyhow!(
            "Ambiguous staged target: {} matched tracked entries",
            path
        ));
    }
    let live_exact = exact
        .iter()
        .filter(|(_, entry)| entry.status != ChangeStatus::Deleted)
        .cloned()
        .collect::<Vec<_>>();
    let deleted_exact = entries
        .iter()
        .any(|(entry_path, entry)| entry_path == path && entry.status == ChangeStatus::Deleted);
    if live_exact.len() > 1 || renamed.len() > 1 {
        return Err(anyhow!(
            "Ambiguous staged target: {} matched multiple live entries",
            path
        ));
    }
    if deleted_exact && renamed.len() == 1 {
        return Ok(renamed.into_iter().next());
    }
    if !deleted_exact && !live_exact.is_empty() && !renamed.is_empty() {
        return Err(anyhow!(
            "Ambiguous staged target: {} matched reused path and rename successor",
            path
        ));
    }
    if let Some(entry) = live_exact.into_iter().next() {
        return Ok(Some(entry));
    }
    if let Some(entry) = renamed.into_iter().next() {
        return Ok(Some(entry));
    }
    Ok((exact.len() == 1).then(|| exact[0].clone()))
}

#[cfg(test)]
#[path = "staging_target_test.rs"]
mod tests;
