use super::{
    StagedEntry, get_staged, list_staged_entries, list_staged_entries_for_doc, take_staged,
};
use crate::models::DocId;
use crate::protocol::ScPathTarget;
use crate::source_control::ChangeStatus;
use crate::utils::path::to_forward_slash;
use anyhow::Result;
use redb::Database;

pub fn get_staged_for_target(
    db: &Database,
    target: &ScPathTarget,
) -> Result<Option<(String, StagedEntry)>> {
    let path = to_forward_slash(&target.path);
    if let Some(doc_id) = target.doc_id {
        return resolve_for_doc(db, &path, doc_id);
    }
    if let Some(entry) = get_staged(db, &path)? {
        return Ok(Some((path, entry)));
    }
    resolve_without_doc(db, &path)
}

pub fn take_staged_for_target(
    db: &Database,
    target: &ScPathTarget,
) -> Result<Option<(String, StagedEntry)>> {
    let Some((path, entry)) = get_staged_for_target(db, target)? else {
        return Ok(None);
    };
    let _ = take_staged(db, &path)?;
    Ok(Some((path, entry)))
}

fn resolve_for_doc(
    db: &Database,
    path: &str,
    doc_id: DocId,
) -> Result<Option<(String, StagedEntry)>> {
    Ok(select_entry(
        list_staged_entries_for_doc(db, doc_id)?,
        path,
        Some(doc_id),
    ))
}

fn resolve_without_doc(db: &Database, path: &str) -> Result<Option<(String, StagedEntry)>> {
    Ok(select_entry(list_staged_entries(db)?, path, None))
}

fn select_entry(
    entries: Vec<(String, StagedEntry)>,
    path: &str,
    doc_id: Option<DocId>,
) -> Option<(String, StagedEntry)> {
    if let Some(doc_id) = doc_id {
        return select_entry_for_doc(entries, path, doc_id);
    }
    select_entry_without_doc(entries, path)
}

fn select_entry_for_doc(
    entries: Vec<(String, StagedEntry)>,
    path: &str,
    doc_id: DocId,
) -> Option<(String, StagedEntry)> {
    let exact = entries
        .iter()
        .find(|(entry_path, entry)| entry_path == path && entry.doc_id == Some(doc_id))
        .cloned();
    if exact.is_some() {
        return exact;
    }
    entries
        .iter()
        .find(|(_, entry)| {
            entry.doc_id == Some(doc_id)
                && entry.status != ChangeStatus::Deleted
                && entry.renamed_from.as_deref().map(to_forward_slash) == Some(path.to_string())
        })
        .cloned()
        .or_else(|| {
            entries
                .iter()
                .find(|(_, entry)| {
                    entry.doc_id == Some(doc_id) && entry.status != ChangeStatus::Deleted
                })
                .cloned()
        })
}

fn select_entry_without_doc(
    entries: Vec<(String, StagedEntry)>,
    path: &str,
) -> Option<(String, StagedEntry)> {
    let exact = entries
        .iter()
        .find(|(entry_path, _)| entry_path == path)
        .cloned()
        .into_iter()
        .collect::<Vec<_>>();
    let live_exact = exact
        .iter()
        .filter(|(_, entry)| entry.status != ChangeStatus::Deleted)
        .cloned()
        .collect::<Vec<_>>();
    let renamed = entries
        .iter()
        .filter(|(_, entry)| {
            entry.status != ChangeStatus::Deleted
                && entry.renamed_from.as_deref().map(to_forward_slash) == Some(path.to_string())
        })
        .cloned()
        .collect::<Vec<_>>();
    let deleted_exact = entries
        .iter()
        .any(|(entry_path, entry)| entry_path == path && entry.status == ChangeStatus::Deleted);
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

#[cfg(test)]
mod tests {
    use super::select_entry_without_doc;
    use crate::source_control::{ChangeStatus, staging::StagedEntry};

    #[test]
    fn prefers_rename_successor_when_old_path_is_reused() {
        let entries = vec![
            (
                "notes/old.md".into(),
                StagedEntry {
                    timestamp: 1,
                    doc_id: None,
                    status: ChangeStatus::Deleted,
                    content_hash: String::new(),
                    has_conflict: false,
                    renamed_from: None,
                },
            ),
            (
                "notes/new.md".into(),
                StagedEntry {
                    timestamp: 2,
                    doc_id: None,
                    status: ChangeStatus::Added,
                    content_hash: String::new(),
                    has_conflict: false,
                    renamed_from: Some("notes/old.md".into()),
                },
            ),
            (
                "notes/old.md".into(),
                StagedEntry {
                    timestamp: 3,
                    doc_id: None,
                    status: ChangeStatus::Added,
                    content_hash: String::new(),
                    has_conflict: false,
                    renamed_from: None,
                },
            ),
        ];

        assert_eq!(
            select_entry_without_doc(entries, "notes/old.md")
                .expect("rename successor should win")
                .0,
            "notes/new.md"
        );
    }

    #[test]
    fn fails_closed_when_path_only_target_is_ambiguous() {
        let entries = vec![
            (
                "notes/old.md".into(),
                StagedEntry {
                    timestamp: 1,
                    doc_id: None,
                    status: ChangeStatus::Added,
                    content_hash: String::new(),
                    has_conflict: false,
                    renamed_from: None,
                },
            ),
            (
                "notes/new.md".into(),
                StagedEntry {
                    timestamp: 2,
                    doc_id: None,
                    status: ChangeStatus::Added,
                    content_hash: String::new(),
                    has_conflict: false,
                    renamed_from: Some("notes/old.md".into()),
                },
            ),
        ];

        assert!(select_entry_without_doc(entries, "notes/old.md").is_none());
    }
}
