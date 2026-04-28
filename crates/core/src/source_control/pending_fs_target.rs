//! plan_ref:
//!   - 04_storage#internal-path-normalization
//!   - 04_storage#watcher-contract
//!   - 07_diff_logic#source-control-runtime

use super::{PendingFsEntry, list_all, list_for_doc, remove};
use crate::models::DocId;
use crate::protocol::ScPathTarget;
use crate::source_control::ChangeStatus;
use crate::utils::path::to_forward_slash;
use anyhow::{Result, anyhow};
use redb::Database;

pub fn get_for_target(db: &Database, target: &ScPathTarget) -> Result<Option<PendingFsEntry>> {
    let path = to_forward_slash(&target.path);
    if let Some(doc_id) = target.doc_id {
        return resolve_for_doc(db, &path, doc_id);
    }
    resolve_without_doc(db, &path)
}

pub fn take_for_target(db: &Database, target: &ScPathTarget) -> Result<Option<PendingFsEntry>> {
    let Some(entry) = get_for_target(db, target)? else {
        return Ok(None);
    };
    remove(db, &entry.path)?;
    Ok(Some(entry))
}

fn resolve_for_doc(db: &Database, path: &str, doc_id: DocId) -> Result<Option<PendingFsEntry>> {
    select_entry(list_for_doc(db, doc_id)?, path, Some(doc_id))
}

fn resolve_without_doc(db: &Database, path: &str) -> Result<Option<PendingFsEntry>> {
    select_entry(list_all(db)?, path, None)
}

fn select_entry(
    entries: Vec<PendingFsEntry>,
    path: &str,
    doc_id: Option<DocId>,
) -> Result<Option<PendingFsEntry>> {
    if let Some(doc_id) = doc_id {
        return Ok(select_entry_for_doc(entries, path, doc_id));
    }
    select_entry_without_doc(entries, path)
}

fn select_entry_for_doc(
    entries: Vec<PendingFsEntry>,
    path: &str,
    doc_id: DocId,
) -> Option<PendingFsEntry> {
    let exact = entries
        .iter()
        .find(|entry| entry.path == path && entry.doc_id == Some(doc_id))
        .cloned();
    if exact.is_some() {
        return exact;
    }
    entries
        .iter()
        .find(|entry| {
            entry.doc_id == Some(doc_id)
                && entry.status_is_live()
                && entry.renamed_from.as_deref().map(to_forward_slash) == Some(path.to_string())
        })
        .cloned()
        .or(exact)
}

fn select_entry_without_doc(
    entries: Vec<PendingFsEntry>,
    path: &str,
) -> Result<Option<PendingFsEntry>> {
    let exact = entries
        .iter()
        .filter(|entry| entry.path == path)
        .cloned()
        .collect::<Vec<_>>();
    let renamed = entries
        .iter()
        .filter(|entry| {
            entry.status_is_live()
                && entry.renamed_from.as_deref().map(to_forward_slash) == Some(path.to_string())
        })
        .cloned()
        .collect::<Vec<_>>();
    if exact
        .iter()
        .chain(renamed.iter())
        .any(|entry| entry.doc_id.is_some())
    {
        return Err(anyhow!(
            "Ambiguous pending_fs target: {} matched tracked entries",
            path
        ));
    }
    let live_exact = exact
        .iter()
        .filter(|entry| entry.status_is_live())
        .cloned()
        .collect::<Vec<_>>();
    let deleted_exact = exact
        .iter()
        .any(|entry| entry.change_type == ChangeStatus::Deleted);
    if live_exact.len() > 1 || renamed.len() > 1 {
        return Err(anyhow!(
            "Ambiguous pending_fs target: {} matched multiple live entries",
            path
        ));
    }
    if deleted_exact && renamed.len() == 1 {
        return Ok(renamed.into_iter().next());
    }
    if !deleted_exact && !live_exact.is_empty() && !renamed.is_empty() {
        return Err(anyhow!(
            "Ambiguous pending_fs target: {} matched reused path and rename successor",
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

trait PendingEntryStatus {
    fn status_is_live(&self) -> bool;
}

impl PendingEntryStatus for PendingFsEntry {
    fn status_is_live(&self) -> bool {
        self.change_type != ChangeStatus::Deleted
    }
}

#[cfg(test)]
#[path = "pending_fs_target_test.rs"]
mod tests;
