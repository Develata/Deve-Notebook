use deve_core::protocol::ScPathTarget;
use deve_core::source_control::ChangeEntry;
use leptos::prelude::*;
use std::collections::HashSet;

pub(super) fn resolve_target(entries: ReadSignal<Vec<ChangeEntry>>, path: &str) -> ScPathTarget {
    let path = deve_core::utils::path::to_forward_slash(path);
    resolve_entry(entries, &path)
        .map(|entry| ScPathTarget {
            path: path.clone(),
            doc_id: entry.doc_id,
        })
        .unwrap_or_else(|| ScPathTarget::from_path(path))
}

pub(super) fn resolve_target_any(
    primary: ReadSignal<Vec<ChangeEntry>>,
    secondary: ReadSignal<Vec<ChangeEntry>>,
    path: &str,
) -> ScPathTarget {
    let path = deve_core::utils::path::to_forward_slash(path);
    resolve_entry(primary, &path)
        .or_else(|| resolve_entry(secondary, &path))
        .map(|entry| ScPathTarget {
            path: path.clone(),
            doc_id: entry.doc_id,
        })
        .unwrap_or_else(|| ScPathTarget::from_path(path))
}

pub(super) fn resolve_targets(
    entries: ReadSignal<Vec<ChangeEntry>>,
    paths: Vec<String>,
) -> Vec<ScPathTarget> {
    let mut seen = HashSet::new();
    paths
        .into_iter()
        .map(|path| resolve_target(entries, &path))
        .filter(|target| !target.path.is_empty())
        .filter(|target| seen.insert(target.path.clone()))
        .collect()
}

fn resolve_entry(entries: ReadSignal<Vec<ChangeEntry>>, path: &str) -> Option<ChangeEntry> {
    entries
        .get_untracked()
        .into_iter()
        .find(|entry| deve_core::utils::path::to_forward_slash(&entry.path) == path)
}
