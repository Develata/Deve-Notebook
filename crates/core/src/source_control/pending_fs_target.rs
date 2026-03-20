use super::{PendingFsEntry, get, list_all, list_for_doc, remove};
use crate::models::DocId;
use crate::protocol::ScPathTarget;
use crate::source_control::ChangeStatus;
use crate::utils::path::to_forward_slash;
use anyhow::Result;
use redb::Database;

pub fn get_for_target(db: &Database, target: &ScPathTarget) -> Result<Option<PendingFsEntry>> {
    let path = to_forward_slash(&target.path);
    if let Some(doc_id) = target.doc_id {
        return resolve_for_doc(db, &path, doc_id);
    }
    if let Some(entry) = get(db, &path)?.filter(|entry| entry.doc_id.is_none()) {
        return Ok(Some(entry));
    }
    resolve_without_doc(db, &path)
}

pub fn take_for_target(db: &Database, target: &ScPathTarget) -> Result<Option<PendingFsEntry>> {
    let Some(entry) = get_for_target(db, target)? else {
        return Ok(None);
    };
    remove(db, &entry.path)?;
    Ok(Some(entry))
}

fn resolve_for_doc(db: &Database, path: &str, doc_id: DocId) -> Result<Option<PendingFsEntry>> {
    Ok(select_entry(list_for_doc(db, doc_id)?, path, Some(doc_id)))
}

fn resolve_without_doc(db: &Database, path: &str) -> Result<Option<PendingFsEntry>> {
    Ok(select_entry(list_all(db)?, path, None))
}

fn select_entry(
    entries: Vec<PendingFsEntry>,
    path: &str,
    doc_id: Option<DocId>,
) -> Option<PendingFsEntry> {
    if let Some(doc_id) = doc_id {
        return select_entry_for_doc(entries, path, doc_id);
    }
    select_entry_without_doc(entries, path)
}

fn select_entry_for_doc(
    entries: Vec<PendingFsEntry>,
    path: &str,
    doc_id: DocId,
) -> Option<PendingFsEntry> {
    let exact = entries
        .iter()
        .find(|entry| entry.path == path && entry.doc_id == Some(doc_id))
        .cloned();
    if exact
        .as_ref()
        .is_some_and(PendingEntryStatus::status_is_live)
    {
        return exact;
    }
    entries
        .iter()
        .find(|entry| {
            entry.doc_id == Some(doc_id)
                && entry.status_is_live()
                && entry.renamed_from.as_deref().map(to_forward_slash) == Some(path.to_string())
        })
        .cloned()
        .or(exact)
        .or_else(|| {
            entries
                .iter()
                .find(|entry| entry.doc_id == Some(doc_id) && entry.status_is_live())
                .cloned()
        })
}

fn select_entry_without_doc(entries: Vec<PendingFsEntry>, path: &str) -> Option<PendingFsEntry> {
    let exact = entries
        .iter()
        .filter(|entry| entry.path == path)
        .cloned()
        .collect::<Vec<_>>();
    let renamed = entries
        .iter()
        .filter(|entry| {
            entry.status_is_live()
                && entry.renamed_from.as_deref().map(to_forward_slash) == Some(path.to_string())
        })
        .cloned()
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
        .filter(|entry| entry.status_is_live())
        .cloned()
        .collect::<Vec<_>>();
    let deleted_exact = exact
        .iter()
        .any(|entry| entry.change_type == ChangeStatus::Deleted);
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
    (exact.len() == 1).then(|| exact[0].clone())
}

trait PendingEntryStatus {
    fn status_is_live(&self) -> bool;
}

impl PendingEntryStatus for PendingFsEntry {
    fn status_is_live(&self) -> bool {
        self.change_type != ChangeStatus::Deleted
    }
}

#[cfg(test)]
mod tests {
    use super::select_entry_without_doc;
    use crate::models::DocId;
    use crate::source_control::ChangeStatus;
    use crate::source_control::pending_fs::PendingFsEntry;

    #[test]
    fn prefers_rename_successor_when_old_path_is_reused() {
        let entries = vec![
            PendingFsEntry {
                path: "notes/old.md".into(),
                renamed_from: None,
                doc_id: None,
                change_type: ChangeStatus::Deleted,
                content_hash: String::new(),
                detected_at: 1,
                has_conflict: false,
            },
            PendingFsEntry {
                path: "notes/new.md".into(),
                renamed_from: Some("notes/old.md".into()),
                doc_id: None,
                change_type: ChangeStatus::Added,
                content_hash: String::new(),
                detected_at: 2,
                has_conflict: false,
            },
            PendingFsEntry {
                path: "notes/old.md".into(),
                renamed_from: None,
                doc_id: None,
                change_type: ChangeStatus::Added,
                content_hash: String::new(),
                detected_at: 3,
                has_conflict: false,
            },
        ];

        assert_eq!(
            select_entry_without_doc(entries, "notes/old.md")
                .expect("rename successor should win")
                .path,
            "notes/new.md"
        );
    }

    #[test]
    fn fails_closed_when_path_only_target_is_ambiguous() {
        let entries = vec![
            PendingFsEntry {
                path: "notes/old.md".into(),
                renamed_from: None,
                doc_id: None,
                change_type: ChangeStatus::Added,
                content_hash: String::new(),
                detected_at: 1,
                has_conflict: false,
            },
            PendingFsEntry {
                path: "notes/new.md".into(),
                renamed_from: Some("notes/old.md".into()),
                doc_id: None,
                change_type: ChangeStatus::Added,
                content_hash: String::new(),
                detected_at: 2,
                has_conflict: false,
            },
        ];

        assert!(select_entry_without_doc(entries, "notes/old.md").is_none());
    }

    #[test]
    fn fails_closed_when_path_only_target_matches_tracked_entries() {
        let doc_id = DocId(uuid::Uuid::nil());
        let entries = vec![
            PendingFsEntry {
                path: "notes/old.md".into(),
                renamed_from: None,
                doc_id: Some(doc_id),
                change_type: ChangeStatus::Deleted,
                content_hash: String::new(),
                detected_at: 1,
                has_conflict: false,
            },
            PendingFsEntry {
                path: "notes/new.md".into(),
                renamed_from: Some("notes/old.md".into()),
                doc_id: Some(doc_id),
                change_type: ChangeStatus::Added,
                content_hash: String::new(),
                detected_at: 2,
                has_conflict: false,
            },
        ];

        assert!(select_entry_without_doc(entries, "notes/old.md").is_none());
    }
}
