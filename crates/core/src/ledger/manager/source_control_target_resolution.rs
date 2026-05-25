//! plan_ref:
//!   - 05_diff_logic#source-control-runtime
//!   - 04_repository#repo-selector-resolution-contract
//!   - 03_storage#internal-path-normalization
//!
//! # Source Control Target Resolution
//!
//! Pure target-resolution rules over already-collected source-control entries.

use crate::models::DocId;
use crate::source_control::{ChangeEntry, ChangeStatus};
use crate::utils::path::to_forward_slash;

pub(super) fn resolve_from_entries(
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

pub(super) fn has_tracked_path_only_candidates(entries: &[ChangeEntry], path: &str) -> bool {
    entries.iter().any(|entry| {
        entry.doc_id.is_some()
            && (to_forward_slash(&entry.path) == path
                || entry
                    .renamed_from
                    .as_ref()
                    .is_some_and(|old_path| to_forward_slash(old_path) == path))
    })
}

pub(super) fn change_identity_key(
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
mod tests;
