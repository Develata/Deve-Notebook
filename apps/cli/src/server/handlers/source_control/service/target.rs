//! plan_ref:
//!   - 07_diff_logic#source-control-runtime

use crate::server::handlers::source_control::present;
use deve_core::models::DocId;
use deve_core::protocol::{ScPathTarget, ServerError, ServerErrorCode};
use deve_core::source_control::{ChangeEntry, ChangeStatus};
use deve_core::utils::path::to_forward_slash;
use std::collections::HashSet;

pub fn related_targets(
    entries: &[ChangeEntry],
    target: &ScPathTarget,
) -> super::ScResult<Vec<ScPathTarget>> {
    let path = match present::resolve_target_path_strict(entries, target) {
        Ok(Some(path)) => path,
        Ok(None) => {
            return Err(ServerError::with_detail(
                ServerErrorCode::ScConflictTargetMissing,
                format!(
                    "Source control target not found in current change set: {}",
                    target.path
                ),
            ));
        }
        Err(err) => {
            return Err(ServerError::with_detail(
                ServerErrorCode::StorageConflict,
                err.to_string(),
            ));
        }
    };
    let resolved = ScPathTarget {
        doc_id: target.doc_id.or_else(|| {
            entries
                .iter()
                .find(|entry| to_forward_slash(&entry.path) == path)
                .and_then(|entry| entry.doc_id)
        }),
        path,
    };
    related_paths(entries, &resolved)?
        .into_iter()
        .map(|path| {
            Ok(ScPathTarget {
                doc_id: resolved.doc_id,
                path,
            })
        })
        .collect()
}

pub fn resolve_target(
    entries: &[ChangeEntry],
    target: &ScPathTarget,
) -> super::ScResult<ScPathTarget> {
    let path = match present::resolve_target_path_strict(entries, target) {
        Ok(Some(path)) => path,
        Ok(None) => {
            return Err(ServerError::with_detail(
                ServerErrorCode::ScConflictTargetMissing,
                format!(
                    "Source control target not found in current change set: {}",
                    target.path
                ),
            ));
        }
        Err(err) => {
            return Err(ServerError::with_detail(
                ServerErrorCode::StorageConflict,
                err.to_string(),
            ));
        }
    };
    let resolved = ScPathTarget {
        doc_id: target.doc_id.or_else(|| {
            entries
                .iter()
                .find(|entry| to_forward_slash(&entry.path) == path)
                .and_then(|entry| entry.doc_id)
        }),
        path,
    };
    if target_exists(entries, &resolved) {
        return Ok(resolved);
    }
    Err(ServerError::with_detail(
        ServerErrorCode::ScConflictTargetMissing,
        format!(
            "Source control target not found in current change set: {}",
            target.path
        ),
    ))
}

pub fn resolved_target_entry<'a>(
    entries: &'a [ChangeEntry],
    resolved: &ScPathTarget,
) -> super::ScResult<&'a ChangeEntry> {
    current_entry(entries, resolved)
}

pub fn resolve_targets(
    entries: &[ChangeEntry],
    targets: Vec<ScPathTarget>,
) -> super::ScResult<Vec<ScPathTarget>> {
    let mut seen = HashSet::new();
    let resolved = targets
        .into_iter()
        .map(|target| resolve_target(entries, &target))
        .collect::<super::ScResult<Vec<_>>>()?;
    Ok(resolved
        .into_iter()
        .filter(|target| !target.path.is_empty())
        .filter(|target| seen.insert((target.doc_id, target.path.clone())))
        .collect())
}

fn target_exists(entries: &[ChangeEntry], target: &ScPathTarget) -> bool {
    let target_path = to_forward_slash(&target.path);
    entries.iter().any(|entry| {
        to_forward_slash(&entry.path) == target_path
            && match target.doc_id {
                Some(doc_id) => entry.doc_id == Some(doc_id),
                None => entry.doc_id.is_none(),
            }
    })
}

fn related_paths(entries: &[ChangeEntry], resolved: &ScPathTarget) -> super::ScResult<Vec<String>> {
    let current = current_entry(entries, resolved)?;
    let mut paths = vec![to_forward_slash(&resolved.path)];
    if let Some(old_path) = current.renamed_from.as_ref() {
        paths.push(to_forward_slash(old_path));
    } else if current.status == ChangeStatus::Deleted
        && let Some(doc_id) = current.doc_id
        && let Some(added) = unique_added_rename_successor(entries, doc_id, &current.path)?
    {
        paths.push(to_forward_slash(&added.path));
    }
    paths.sort();
    paths.dedup();
    Ok(paths)
}

fn current_entry<'a>(
    entries: &'a [ChangeEntry],
    resolved: &ScPathTarget,
) -> super::ScResult<&'a ChangeEntry> {
    let path = to_forward_slash(&resolved.path);
    let matches = entries
        .iter()
        .filter(|entry| {
            to_forward_slash(&entry.path) == path
                && match resolved.doc_id {
                    Some(doc_id) => entry.doc_id == Some(doc_id),
                    None => entry.doc_id.is_none(),
                }
        })
        .collect::<Vec<_>>();
    unique_match(matches, "current source control target", &path)
}

fn unique_added_rename_successor<'a>(
    entries: &'a [ChangeEntry],
    doc_id: DocId,
    old_path: &str,
) -> super::ScResult<Option<&'a ChangeEntry>> {
    let old_path = to_forward_slash(old_path);
    let matches = entries
        .iter()
        .filter(|entry| {
            entry.status == ChangeStatus::Added
                && entry.doc_id == Some(doc_id)
                && entry.renamed_from.as_deref().map(to_forward_slash) == Some(old_path.clone())
        })
        .collect::<Vec<_>>();
    if matches.is_empty() {
        return Ok(None);
    }
    Ok(Some(unique_match(
        matches,
        "source control rename successor",
        &old_path,
    )?))
}

fn unique_match<'a>(
    matches: Vec<&'a ChangeEntry>,
    label: &str,
    path: &str,
) -> super::ScResult<&'a ChangeEntry> {
    match matches.as_slice() {
        [entry] => Ok(*entry),
        [] => Err(ServerError::with_detail(
            ServerErrorCode::ScConflictTargetMissing,
            format!(
                "Source control target not found in current change set: {}",
                path
            ),
        )),
        _ => Err(ServerError::with_detail(
            ServerErrorCode::StorageConflict,
            format!("Ambiguous {}: {}", label, path),
        )),
    }
}

#[cfg(test)]
#[path = "target_test.rs"]
mod tests;
