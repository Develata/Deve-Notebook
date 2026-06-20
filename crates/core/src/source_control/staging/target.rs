//! plan_ref:
//!   - 03_storage/index#internal-path-normalization
//!   - 05_diff_logic#source-control-runtime

use super::{list_staged_entries, list_staged_entries_for_doc, take_staged, StagedEntry};
use crate::models::DocId;
use crate::protocol::ScPathTarget;
use crate::source_control::ChangeStatus;
use crate::utils::path::to_forward_slash;
use anyhow::{anyhow, Result};
use redb::Database;

pub fn get_staged_for_target(
    db: &Database,
    target: &ScPathTarget,
) -> Result<Option<(String, StagedEntry)>> {
    let path = to_forward_slash(&target.path);
    if let Some(doc_id) = target.doc_id {
        return resolve_for_doc(db, &path, doc_id);
    }
    resolve_without_doc(db, &path)
}

pub fn take_staged_for_target(
    db: &Database,
    target: &ScPathTarget,
) -> Result<Option<(String, StagedEntry)>> {
    // Unstage precisely consumes a staged entry: an exact staged path match for
    // this target wins over rename-successor resolution, so unstaging the deleted
    // side of a rename keeps its exact path/status instead of consuming the added
    // counterpart (which would risk a non-atomic half-migration). Stage and
    // read paths keep "live successor wins" via get_staged_for_target.
    let resolved = match exact_staged_for_target(db, target)? {
        Some(hit) => Some(hit),
        None => get_staged_for_target(db, target)?,
    };
    let Some((path, entry)) = resolved else {
        return Ok(None);
    };
    let _ = take_staged(db, &path)?;
    Ok(Some((path, entry)))
}

fn exact_staged_for_target(
    db: &Database,
    target: &ScPathTarget,
) -> Result<Option<(String, StagedEntry)>> {
    // Only doc-scoped targets get exact-path preference: the caller has precisely
    // identified the entry by (path, doc_id). Path-only targets keep ambiguity
    // fail-closed via get_staged_for_target.
    let Some(doc_id) = target.doc_id else {
        return Ok(None);
    };
    let path = to_forward_slash(&target.path);
    Ok(list_staged_entries_for_doc(db, doc_id)?
        .into_iter()
        .find(|(entry_path, _)| *entry_path == path))
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
    if exact
        .as_ref()
        .is_some_and(|(_, entry)| entry.status != ChangeStatus::Deleted)
    {
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
        .or(exact)
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
mod tests;
