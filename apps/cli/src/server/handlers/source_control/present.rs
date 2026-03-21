use anyhow::{Result, anyhow};
use deve_core::protocol::ScPathTarget;
use deve_core::source_control::{ChangeEntry, ChangeStatus};
use std::collections::HashSet;

pub fn collapse_rename_candidates(entries: Vec<ChangeEntry>) -> Vec<ChangeEntry> {
    let hidden: HashSet<(Option<deve_core::models::DocId>, String)> = entries
        .iter()
        .filter_map(|entry| {
            entry.renamed_from.as_ref().map(|old_path| {
                (
                    entry.doc_id,
                    deve_core::utils::path::to_forward_slash(old_path),
                )
            })
        })
        .collect();

    entries
        .into_iter()
        .filter(|entry| {
            !(entry.status == ChangeStatus::Deleted
                && hidden.contains(&(entry.doc_id, normalized(&entry.path))))
        })
        .collect()
}

pub fn expand_related_paths(entries: &[ChangeEntry], path: &str) -> Vec<String> {
    let path = normalized(path);
    let Some(current) = entries.iter().find(|entry| normalized(&entry.path) == path) else {
        return vec![path];
    };
    let mut paths = vec![path];
    if let Some(old_path) = current.renamed_from.as_ref() {
        paths.push(normalized(old_path));
    } else if current.status == ChangeStatus::Deleted
        && let Some(doc_id) = current.doc_id
        && let Some(added) = entries.iter().find(|entry| {
            entry.status == ChangeStatus::Added
                && entry.doc_id == Some(doc_id)
                && entry.renamed_from.as_deref() == Some(current.path.as_str())
        })
    {
        paths.push(normalized(&added.path));
    }
    paths.sort();
    paths.dedup();
    paths
}

#[cfg_attr(not(test), allow(dead_code))]
pub fn resolve_target(entries: &[ChangeEntry], target: &ScPathTarget) -> ScPathTarget {
    let Some(path) = resolve_target_path(entries, target) else {
        return target.clone();
    };
    let doc_id = target.doc_id.or_else(|| {
        entries
            .iter()
            .find(|entry| normalized(&entry.path) == path)
            .and_then(|entry| entry.doc_id)
    });
    ScPathTarget { path, doc_id }
}

#[cfg_attr(not(test), allow(dead_code))]
pub fn expand_related_targets(entries: &[ChangeEntry], target: &ScPathTarget) -> Vec<ScPathTarget> {
    let resolved = resolve_target(entries, target);
    if resolve_target_path(entries, &resolved).is_none() {
        return vec![];
    }
    let mut targets = vec![];
    for path in expand_related_paths(entries, &resolved.path) {
        let doc_id = entries
            .iter()
            .find(|entry| normalized(&entry.path) == path)
            .and_then(|entry| entry.doc_id)
            .or(resolved.doc_id);
        let candidate = ScPathTarget { path, doc_id };
        if !targets.contains(&candidate) {
            targets.push(candidate);
        }
    }
    targets
}

#[cfg_attr(not(test), allow(dead_code))]
pub fn resolve_target_path(entries: &[ChangeEntry], target: &ScPathTarget) -> Option<String> {
    resolve_target_path_strict(entries, target).ok().flatten()
}

pub(crate) fn resolve_target_path_strict(
    entries: &[ChangeEntry],
    target: &ScPathTarget,
) -> Result<Option<String>> {
    let path = normalized(&target.path);
    if let Some(doc_id) = target.doc_id {
        return Ok(entries
            .iter()
            .find(|entry| {
                entry.doc_id == Some(doc_id)
                    && normalized(&entry.path) == path
                    && entry.status != ChangeStatus::Deleted
            })
            .or_else(|| {
                entries.iter().find(|entry| {
                    entry.doc_id == Some(doc_id) && entry.status != ChangeStatus::Deleted
                })
            })
            .or_else(|| {
                entries
                    .iter()
                    .find(|entry| entry.doc_id == Some(doc_id) && normalized(&entry.path) == path)
            })
            .or_else(|| entries.iter().find(|entry| entry.doc_id == Some(doc_id)))
            .map(|entry| normalized(&entry.path)));
    }
    Ok(resolve_without_doc_id(entries, &path)?.map(|entry| normalized(&entry.path)))
}

fn resolve_without_doc_id<'a>(
    entries: &'a [ChangeEntry],
    path: &str,
) -> Result<Option<&'a ChangeEntry>> {
    let exact = entries
        .iter()
        .filter(|entry| normalized(&entry.path) == path)
        .collect::<Vec<_>>();
    let renamed = entries
        .iter()
        .filter(|entry| {
            entry.status != ChangeStatus::Deleted
                && entry
                    .renamed_from
                    .as_ref()
                    .is_some_and(|old_path| normalized(old_path) == path)
        })
        .collect::<Vec<_>>();
    if exact
        .iter()
        .chain(renamed.iter())
        .any(|entry| entry.doc_id.is_some())
    {
        return Err(anyhow!(
            "Ambiguous source control target: {} matched tracked entries",
            path
        ));
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
        return Err(anyhow!(
            "Ambiguous source control target: {} matched multiple live entries",
            path
        ));
    }
    if deleted_exact && renamed.len() == 1 {
        return Ok(renamed.into_iter().next());
    }
    if !deleted_exact && !live_exact.is_empty() && !renamed.is_empty() {
        return Err(anyhow!(
            "Ambiguous source control target: {} matched reused path and rename successor",
            path
        ));
    }
    if let Some(entry) = live_exact.into_iter().next() {
        return Ok(Some(entry));
    }
    if let Some(entry) = renamed.into_iter().next() {
        return Ok(Some(entry));
    }
    Ok((exact.len() == 1).then_some(exact[0]))
}

fn normalized(path: &str) -> String {
    deve_core::utils::path::to_forward_slash(path)
}

#[cfg(test)]
#[path = "present_test.rs"]
mod tests;
