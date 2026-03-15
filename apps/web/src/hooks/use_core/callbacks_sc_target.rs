use deve_core::protocol::ScPathTarget;
use deve_core::source_control::ChangeEntry;
use std::collections::HashSet;

pub(super) fn to_target(entry: &ChangeEntry) -> ScPathTarget {
    ScPathTarget {
        path: normalized(&entry.path),
        doc_id: entry.doc_id,
    }
}

pub(super) fn to_targets(entries: Vec<ChangeEntry>) -> Vec<ScPathTarget> {
    let mut seen = HashSet::new();
    entries
        .into_iter()
        .map(|entry| to_target(&entry))
        .filter(|target| !target.path.is_empty())
        .filter(|target| seen.insert((target.doc_id, target.path.clone())))
        .collect()
}

fn normalized(path: &str) -> String {
    deve_core::utils::path::to_forward_slash(path)
}

#[cfg(test)]
mod tests {
    use super::{to_target, to_targets};
    use deve_core::models::DocId;
    use deve_core::source_control::{ChangeEntry, ChangeStatus};

    #[test]
    fn to_target_preserves_doc_id_and_normalizes_path() {
        let doc_id = DocId::new();
        let target = to_target(&ChangeEntry {
            path: "notes\\a.md".into(),
            renamed_from: None,
            doc_id: Some(doc_id),
            status: ChangeStatus::Modified,
            has_conflict: false,
        });

        assert_eq!(target.path, "notes/a.md");
        assert_eq!(target.doc_id, Some(doc_id));
    }

    #[test]
    fn to_targets_keeps_distinct_doc_ids_for_same_path() {
        let first = DocId::new();
        let second = DocId::new();
        let targets = to_targets(vec![
            ChangeEntry {
                path: "notes/a.md".into(),
                renamed_from: None,
                doc_id: Some(first),
                status: ChangeStatus::Modified,
                has_conflict: false,
            },
            ChangeEntry {
                path: "notes/a.md".into(),
                renamed_from: None,
                doc_id: Some(second),
                status: ChangeStatus::Added,
                has_conflict: false,
            },
            ChangeEntry {
                path: "notes/a.md".into(),
                renamed_from: None,
                doc_id: Some(second),
                status: ChangeStatus::Added,
                has_conflict: false,
            },
        ]);

        assert_eq!(targets.len(), 2);
        assert!(targets.iter().any(|target| target.doc_id == Some(first)));
        assert!(targets.iter().any(|target| target.doc_id == Some(second)));
    }
}
