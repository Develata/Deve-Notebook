//! plan_ref:
//!   - 05_diff_logic#source-control-runtime
//!   - 03_storage/authority#facts-partition
//!
use crate::models::DocId;
use crate::source_control::{ChangeStatus, staging::StagedEntry};
use std::collections::HashMap;

#[derive(Clone, Hash, PartialEq, Eq)]
enum CommitKey {
    Tracked(DocId),
    Untracked(String),
}

pub(super) struct CommitTarget {
    pub path: String,
    pub renamed_from: Option<String>,
    pub doc_id: Option<DocId>,
    pub delete_only: bool,
    pub has_rename_evidence: bool,
    pub allow_confirmed_overlap: bool,
}

pub(super) fn build_targets(staged: Vec<(String, StagedEntry)>) -> Vec<CommitTarget> {
    let mut groups: Vec<Vec<(String, StagedEntry)>> = Vec::new();
    let mut indices: HashMap<CommitKey, usize> = HashMap::new();
    for (path, entry) in staged {
        let key = entry
            .doc_id
            .map(CommitKey::Tracked)
            .unwrap_or_else(|| CommitKey::Untracked(path.clone()));
        match indices.get(&key).copied() {
            Some(index) => groups[index].push((path, entry)),
            None => {
                indices.insert(key, groups.len());
                groups.push(vec![(path, entry)]);
            }
        }
    }
    groups.into_iter().map(resolve_group).collect()
}

fn resolve_group(entries: Vec<(String, StagedEntry)>) -> CommitTarget {
    let live = entries
        .iter()
        .find(|(_, entry)| entry.status != ChangeStatus::Deleted);
    let (path, entry) = live.unwrap_or(&entries[0]);
    let has_delete_side = entries
        .iter()
        .any(|(_, entry)| entry.status == ChangeStatus::Deleted);
    let allow_confirmed_overlap = entries.iter().any(|(_, entry)| entry.resolved_conflict);
    CommitTarget {
        path: path.clone(),
        renamed_from: entry.renamed_from.clone(),
        doc_id: entry.doc_id,
        delete_only: entries
            .iter()
            .all(|(_, entry)| entry.status == ChangeStatus::Deleted),
        has_rename_evidence: has_delete_side || entry.renamed_from.is_some(),
        allow_confirmed_overlap,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(doc_id: Option<DocId>, status: ChangeStatus) -> StagedEntry {
        StagedEntry {
            timestamp: 1,
            renamed_from: None,
            doc_id,
            status,
            content_hash: String::new(),
            has_conflict: false,
            resolved_conflict: false,
        }
    }

    #[test]
    fn coalesces_tracked_rename_pair_to_single_target() {
        let doc_id = DocId::new();
        let targets = build_targets(vec![
            (
                "notes/a.md".into(),
                entry(Some(doc_id), ChangeStatus::Deleted),
            ),
            (
                "notes/b.md".into(),
                entry(Some(doc_id), ChangeStatus::Added),
            ),
        ]);
        assert_eq!(targets.len(), 1);
        assert_eq!(targets[0].path, "notes/b.md");
        assert_eq!(targets[0].renamed_from, None);
        assert!(!targets[0].delete_only);
        assert!(targets[0].has_rename_evidence);
    }

    #[test]
    fn marks_resolved_conflict_targets_for_confirmed_overlap_override() {
        let doc_id = DocId::new();
        let mut staged = entry(Some(doc_id), ChangeStatus::Modified);
        staged.resolved_conflict = true;

        let targets = build_targets(vec![("notes/a.md".into(), staged)]);

        assert_eq!(targets.len(), 1);
        assert!(targets[0].allow_confirmed_overlap);
    }
}
