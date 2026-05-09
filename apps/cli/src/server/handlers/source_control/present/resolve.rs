//! plan_ref:
//!   - 07_diff_logic#source-control-runtime

use anyhow::{Result, anyhow};
use deve_core::protocol::ScPathTarget;
use deve_core::source_control::{ChangeEntry, ChangeStatus};

pub(crate) fn resolve_target_path_strict(
    entries: &[ChangeEntry],
    target: &ScPathTarget,
) -> Result<Option<String>> {
    let path = normalized(&target.path);
    if let Some(doc_id) = target.doc_id {
        return Ok(
            resolve_with_doc_id(entries, &path, doc_id)?.map(|entry| normalized(&entry.path))
        );
    }
    Ok(resolve_without_doc_id(entries, &path)?.map(|entry| normalized(&entry.path)))
}

fn resolve_with_doc_id<'a>(
    entries: &'a [ChangeEntry],
    path: &str,
    doc_id: deve_core::models::DocId,
) -> Result<Option<&'a ChangeEntry>> {
    let exact = entries
        .iter()
        .filter(|entry| entry.doc_id == Some(doc_id) && normalized(&entry.path) == path)
        .collect::<Vec<_>>();
    let live_exact = exact
        .iter()
        .copied()
        .filter(|entry| entry.status != ChangeStatus::Deleted)
        .collect::<Vec<_>>();
    let renamed = entries
        .iter()
        .filter(|entry| matches_doc_rename_successor(entry, path, doc_id))
        .collect::<Vec<_>>();

    if live_exact.len() > 1 {
        return Err(anyhow!(
            "Ambiguous source control target: {} matched multiple exact doc entries",
            path
        ));
    }
    if renamed.len() > 1 {
        return Err(anyhow!(
            "Ambiguous source control target: {} matched multiple doc rename successors",
            path
        ));
    }
    if exact.len() > 1 && live_exact.is_empty() {
        return Err(anyhow!(
            "Ambiguous source control target: {} matched multiple deleted doc entries",
            path
        ));
    }
    if live_exact.len() == 1 && !renamed.is_empty() {
        return Err(anyhow!(
            "Ambiguous source control target: {} matched exact doc entry and rename successor",
            path
        ));
    }
    if let Some(entry) = live_exact.into_iter().next() {
        return Ok(Some(entry));
    }
    if let Some(entry) = renamed.into_iter().next() {
        return Ok(Some(entry));
    }
    Ok((exact.len() == 1).then(|| exact[0]))
}

fn matches_doc_rename_successor(
    entry: &ChangeEntry,
    path: &str,
    doc_id: deve_core::models::DocId,
) -> bool {
    entry.doc_id == Some(doc_id)
        && entry.status != ChangeStatus::Deleted
        && entry
            .renamed_from
            .as_ref()
            .is_some_and(|old_path| normalized(old_path) == path)
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
    reject_tracked_or_ambiguous(path, &exact, &renamed)?;
    let live_exact = exact
        .iter()
        .copied()
        .filter(|entry| entry.status != ChangeStatus::Deleted)
        .collect::<Vec<_>>();
    resolve_untracked_path(path, exact, renamed, live_exact)
}

fn reject_tracked_or_ambiguous(
    path: &str,
    exact: &[&ChangeEntry],
    renamed: &[&ChangeEntry],
) -> Result<()> {
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
    if exact
        .iter()
        .filter(|entry| entry.status != ChangeStatus::Deleted)
        .count()
        > 1
        || renamed.len() > 1
    {
        return Err(anyhow!(
            "Ambiguous source control target: {} matched multiple live entries",
            path
        ));
    }
    Ok(())
}

fn resolve_untracked_path<'a>(
    path: &str,
    exact: Vec<&'a ChangeEntry>,
    renamed: Vec<&'a ChangeEntry>,
    live_exact: Vec<&'a ChangeEntry>,
) -> Result<Option<&'a ChangeEntry>> {
    let deleted_exact = exact
        .iter()
        .any(|entry| entry.status == ChangeStatus::Deleted);
    if deleted_exact && renamed.len() == 1 {
        return Ok(renamed.into_iter().next());
    }
    if !deleted_exact && !live_exact.is_empty() && !renamed.is_empty() {
        return Err(anyhow!(
            "Ambiguous source control target: {} matched reused path and rename successor",
            path
        ));
    }
    Ok(live_exact.into_iter().next().or_else(|| {
        renamed
            .into_iter()
            .next()
            .or_else(|| (exact.len() == 1).then(|| exact[0]))
    }))
}

fn normalized(path: &str) -> String {
    deve_core::utils::path::to_forward_slash(path)
}
