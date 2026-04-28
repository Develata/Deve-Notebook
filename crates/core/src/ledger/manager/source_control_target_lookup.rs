//! plan_ref:
//!   - 07_diff_logic#source-control-runtime
//!   - 06_repository#repo-selector-resolution-contract
//!   - 04_storage#internal-path-normalization
//!
use crate::ledger::RepoManager;
use crate::models::DocId;
use crate::protocol::ScPathTarget;
use crate::source_control::{ChangeEntry, ChangeStatus, pending_fs, staging};
use crate::utils::path::to_forward_slash;
use anyhow::{Result, anyhow};
use redb::Database;
use std::collections::HashSet;

pub(super) fn resolve_change_path(
    repo: &RepoManager,
    repo_name: &str,
    target: &ScPathTarget,
) -> Result<String> {
    let path = to_forward_slash(&target.path);
    if let Some(doc_id) = target.doc_id {
        let entries = repo.run_on_local_repo(repo_name, |db| change_entries(db, doc_id))?;
        if let Some(resolved) = resolve_from_entries(&entries, &path, Some(doc_id)) {
            return Ok(resolved);
        }
        return Err(anyhow!(
            "Source control target not resolved for doc {} at {}",
            doc_id,
            path
        ));
    }
    let entries = repo.list_changes_in_local_repo(repo_name)?;
    if has_tracked_path_only_candidates(&entries, &path) {
        return Err(anyhow!(
            "Tracked source control target requires document identity: {}",
            path
        ));
    }
    if let Some(resolved) = resolve_from_entries(&entries, &path, None) {
        return Ok(resolved);
    }
    Err(anyhow!(
        "Source control target not resolved for path {}",
        path
    ))
}

fn pending_entries(db: &Database, doc_id: DocId) -> Result<Vec<ChangeEntry>> {
    Ok(pending_fs::list_for_doc(db, doc_id)?
        .into_iter()
        .map(|entry| ChangeEntry {
            path: entry.path,
            renamed_from: entry.renamed_from,
            doc_id: entry.doc_id,
            status: entry.change_type,
            has_conflict: entry.has_conflict,
        })
        .collect())
}

fn staged_entries(db: &Database, doc_id: DocId) -> Result<Vec<ChangeEntry>> {
    Ok(staging::list_staged_entries_for_doc(db, doc_id)?
        .into_iter()
        .map(|(path, entry)| ChangeEntry {
            path,
            renamed_from: entry.renamed_from,
            doc_id: entry.doc_id,
            status: entry.status,
            has_conflict: entry.has_conflict,
        })
        .collect())
}

fn change_entries(db: &Database, doc_id: DocId) -> Result<Vec<ChangeEntry>> {
    let staged = staged_entries(db, doc_id)?;
    let staged_keys: HashSet<_> = staged.iter().map(change_identity_key).collect();
    let mut changes = staged;
    changes.extend(
        pending_entries(db, doc_id)?
            .into_iter()
            .filter(|entry| !staged_keys.contains(&change_identity_key(entry))),
    );
    Ok(changes)
}

fn resolve_from_entries(
    entries: &[ChangeEntry],
    path: &str,
    doc_id: Option<DocId>,
) -> Option<String> {
    let by_doc = doc_id.and_then(|doc_id| resolve_for_doc(entries, path, doc_id));
    by_doc
        .or_else(|| resolve_without_doc_id(entries, path))
        .map(|entry| to_forward_slash(&entry.path))
}

fn resolve_for_doc<'a>(
    entries: &'a [ChangeEntry],
    path: &str,
    doc_id: DocId,
) -> Option<&'a ChangeEntry> {
    let exact = entries
        .iter()
        .find(|entry| entry.doc_id == Some(doc_id) && to_forward_slash(&entry.path) == path);
    if exact.is_some_and(|entry| entry.status != ChangeStatus::Deleted) {
        return exact;
    }
    entries
        .iter()
        .find(|entry| {
            entry.doc_id == Some(doc_id)
                && entry.status != ChangeStatus::Deleted
                && entry
                    .renamed_from
                    .as_ref()
                    .is_some_and(|old_path| to_forward_slash(old_path) == path)
        })
        .or(exact)
}

fn resolve_without_doc_id<'a>(entries: &'a [ChangeEntry], path: &str) -> Option<&'a ChangeEntry> {
    let exact = entries
        .iter()
        .filter(|entry| to_forward_slash(&entry.path) == path)
        .collect::<Vec<_>>();
    let renamed = entries
        .iter()
        .filter(|entry| {
            entry.status != ChangeStatus::Deleted
                && entry
                    .renamed_from
                    .as_ref()
                    .is_some_and(|old_path| to_forward_slash(old_path) == path)
        })
        .collect::<Vec<_>>();
    if exact
        .iter()
        .chain(renamed.iter())
        .any(|entry| entry.doc_id.is_some())
    {
        return None;
    }
    let live_exact = exact
        .iter()
        .copied()
        .filter(|entry| entry.status != ChangeStatus::Deleted)
        .collect::<Vec<_>>();
    let deleted_exact = exact
        .iter()
        .any(|entry| entry.status == ChangeStatus::Deleted);
    if live_exact.len() > 1 || renamed.len() > 1 {
        return None;
    }
    if deleted_exact && renamed.len() == 1 {
        return renamed.into_iter().next();
    }
    if !deleted_exact && !live_exact.is_empty() && !renamed.is_empty() {
        return None;
    }
    if let Some(entry) = live_exact.into_iter().next() {
        return Some(entry);
    }
    if let Some(entry) = renamed.into_iter().next() {
        return Some(entry);
    }
    (exact.len() == 1)
        .then(|| exact.into_iter().next())
        .flatten()
}

fn has_tracked_path_only_candidates(entries: &[ChangeEntry], path: &str) -> bool {
    entries.iter().any(|entry| {
        entry.doc_id.is_some()
            && (to_forward_slash(&entry.path) == path
                || entry
                    .renamed_from
                    .as_ref()
                    .is_some_and(|old_path| to_forward_slash(old_path) == path))
    })
}

fn change_identity_key(
    entry: &ChangeEntry,
) -> (Option<DocId>, String, Option<String>, ChangeStatus) {
    (
        entry.doc_id,
        to_forward_slash(&entry.path),
        entry.renamed_from.as_deref().map(to_forward_slash),
        entry.status,
    )
}

#[cfg(test)]
#[path = "source_control_target_lookup_test.rs"]
mod tests;
