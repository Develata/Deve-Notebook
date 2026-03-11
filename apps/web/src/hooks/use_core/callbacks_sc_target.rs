use deve_core::protocol::ScPathTarget;
use deve_core::source_control::ChangeEntry;
use leptos::prelude::*;
use std::collections::HashSet;

pub(super) fn resolve_target(entries: ReadSignal<Vec<ChangeEntry>>, path: &str) -> ScPathTarget {
    let path = deve_core::utils::path::to_forward_slash(path);
    resolve_entry(entries, &path)
        .map(to_target)
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
        .map(to_target)
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
        .find(|entry| {
            normalized(&entry.path) == path
                && entry.status != deve_core::source_control::ChangeStatus::Deleted
        })
        .or_else(|| {
            entries.get_untracked().into_iter().find(|entry| {
                entry.status != deve_core::source_control::ChangeStatus::Deleted
                    && entry
                        .renamed_from
                        .as_ref()
                        .is_some_and(|old_path| normalized(old_path) == path)
            })
        })
        .or_else(|| {
            entries
                .get_untracked()
                .into_iter()
                .find(|entry| normalized(&entry.path) == path)
        })
}

fn to_target(entry: ChangeEntry) -> ScPathTarget {
    ScPathTarget {
        path: entry.path,
        doc_id: entry.doc_id,
    }
}

fn normalized(path: &str) -> String {
    deve_core::utils::path::to_forward_slash(path)
}

#[cfg(test)]
mod tests {
    use super::resolve_target;
    use deve_core::source_control::{ChangeEntry, ChangeStatus};
    use leptos::prelude::signal;

    #[test]
    fn resolve_target_maps_old_rename_path_to_current_entry() {
        let (read, _write) = signal(vec![
            ChangeEntry {
                path: "notes/old.md".into(),
                renamed_from: None,
                doc_id: None,
                status: ChangeStatus::Deleted,
                has_conflict: false,
            },
            ChangeEntry {
                path: "notes/new.md".into(),
                renamed_from: Some("notes/old.md".into()),
                doc_id: None,
                status: ChangeStatus::Added,
                has_conflict: false,
            },
        ]);
        assert_eq!(resolve_target(read, "notes/old.md").path, "notes/new.md");
    }
}
