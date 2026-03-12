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

pub fn resolve_target(entries: &[ChangeEntry], target: &ScPathTarget) -> ScPathTarget {
    let path = resolve_target_path(entries, target);
    let doc_id = target.doc_id.or_else(|| {
        entries
            .iter()
            .find(|entry| normalized(&entry.path) == path)
            .and_then(|entry| entry.doc_id)
    });
    ScPathTarget { path, doc_id }
}

pub fn expand_related_targets(entries: &[ChangeEntry], target: &ScPathTarget) -> Vec<ScPathTarget> {
    let resolved = resolve_target(entries, target);
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
        .or_else(|| resolve_without_doc_id(entries, &path))
        .map(|entry| normalized(&entry.path))
        .unwrap_or(path)
}

fn resolve_without_doc_id<'a>(entries: &'a [ChangeEntry], path: &str) -> Option<&'a ChangeEntry> {
    let renamed_successor = entries.iter().find(|entry| {
        entry.status != ChangeStatus::Deleted
            && entry
                .renamed_from
                .as_ref()
                .is_some_and(|old_path| normalized(old_path) == path)
    });
    let path_reused_after_delete = renamed_successor.is_some()
        && entries
            .iter()
            .any(|entry| normalized(&entry.path) == path && entry.status == ChangeStatus::Deleted);
    if path_reused_after_delete {
        return renamed_successor;
    }
    entries
        .iter()
        .find(|entry| normalized(&entry.path) == path && entry.status != ChangeStatus::Deleted)
        .or(renamed_successor)
        .or_else(|| entries.iter().find(|entry| normalized(&entry.path) == path))
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

    #[test]
    fn resolve_target_matches_renamed_from_without_doc_id() {
        let entries = vec![
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
        ];

        assert_eq!(
            resolve_target_path(&entries, &ScPathTarget::from_path("notes/old.md")),
            "notes/new.md"
        );
    }

    #[test]
    fn expand_related_targets_preserves_doc_id_for_rename_pair() {
        let doc_id = DocId(Uuid::nil());
        let entries = vec![
            ChangeEntry {
                path: "notes/old.md".into(),
                renamed_from: None,
                doc_id: Some(doc_id),
                status: ChangeStatus::Deleted,
                has_conflict: false,
            },
            ChangeEntry {
                path: "notes/new.md".into(),
                renamed_from: Some("notes/old.md".into()),
                doc_id: Some(doc_id),
                status: ChangeStatus::Added,
                has_conflict: false,
            },
        ];

        assert_eq!(
            expand_related_targets(&entries, &ScPathTarget::from_path("notes/new.md")),
            vec![
                ScPathTarget {
                    path: "notes/new.md".into(),
                    doc_id: Some(doc_id),
                },
                ScPathTarget {
                    path: "notes/old.md".into(),
                    doc_id: Some(doc_id),
                },
            ]
        );
    }

    #[test]
    fn resolve_target_prefers_rename_successor_over_reused_old_path() {
        let old_doc = DocId(Uuid::nil());
        let new_doc = DocId(Uuid::from_u128(1));
        let entries = vec![
            ChangeEntry {
                path: "notes/old.md".into(),
                renamed_from: None,
                doc_id: Some(old_doc),
                status: ChangeStatus::Deleted,
                has_conflict: false,
            },
            ChangeEntry {
                path: "notes/new.md".into(),
                renamed_from: Some("notes/old.md".into()),
                doc_id: Some(old_doc),
                status: ChangeStatus::Added,
                has_conflict: false,
            },
            ChangeEntry {
                path: "notes/old.md".into(),
                renamed_from: None,
                doc_id: Some(new_doc),
                status: ChangeStatus::Added,
                has_conflict: false,
            },
        ];

        assert_eq!(
            resolve_target_path(&entries, &ScPathTarget::from_path("notes/old.md")),
            "notes/new.md"
        );
    }
}
