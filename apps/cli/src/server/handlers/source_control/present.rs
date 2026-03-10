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

pub fn expand_related_target_paths(entries: &[ChangeEntry], target: &ScPathTarget) -> Vec<String> {
    let path = resolve_target_path(entries, target);
    expand_related_paths(entries, &path)
}

pub fn resolve_target_path(entries: &[ChangeEntry], target: &ScPathTarget) -> String {
    let path = normalized(&target.path);
    target
        .doc_id
        .and_then(|doc_id| {
            entries
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
                    entries.iter().find(|entry| {
                        entry.doc_id == Some(doc_id) && normalized(&entry.path) == path
                    })
                })
                .or_else(|| entries.iter().find(|entry| entry.doc_id == Some(doc_id)))
        })
        .or_else(|| entries.iter().find(|entry| normalized(&entry.path) == path))
        .map(|entry| normalized(&entry.path))
        .unwrap_or(path)
}

fn normalized(path: &str) -> String {
    deve_core::utils::path::to_forward_slash(path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use deve_core::models::DocId;
    use uuid::Uuid;

    #[test]
    fn resolve_target_prefers_doc_id_over_stale_path() {
        let doc_id = DocId(Uuid::nil());
        let entries = vec![ChangeEntry {
            path: "notes/new.md".into(),
            renamed_from: Some("notes/old.md".into()),
            doc_id: Some(doc_id),
            status: ChangeStatus::Added,
            has_conflict: false,
        }];
        let target = ScPathTarget {
            path: "notes/old.md".into(),
            doc_id: Some(doc_id),
        };

        assert_eq!(resolve_target_path(&entries, &target), "notes/new.md");
    }
}
