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
        .cloned();
    let renamed_successor = entries.iter().find(|(_, entry)| {
        entry.status != ChangeStatus::Deleted
            && entry.renamed_from.as_deref().map(to_forward_slash) == Some(path.to_string())
    });
    let path_reused_after_delete = renamed_successor.is_some()
        && entries
            .iter()
            .any(|(entry_path, entry)| entry_path == path && entry.status == ChangeStatus::Deleted);
    if path_reused_after_delete {
        return renamed_successor.cloned();
    }
    if exact.is_some() {
        return exact;
    }
    entries
        .iter()
        .find(|(entry_path, entry)| entry_path == path && entry.status != ChangeStatus::Deleted)
        .cloned()
        .or_else(|| renamed_successor.cloned())
        .or_else(|| {
            entries
                .into_iter()
                .find(|(entry_path, _)| entry_path == path)
        })
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
}
