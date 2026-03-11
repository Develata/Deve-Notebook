use crate::ledger::RepoManager;
use crate::models::DocId;
use crate::protocol::ScPathTarget;
use crate::source_control::{ChangeEntry, ChangeStatus, pending_fs, staging};
use crate::utils::path::to_forward_slash;
use anyhow::Result;
use redb::Database;
use std::collections::HashSet;

#[derive(Clone, Copy)]
pub(super) enum ScTargetScope {
    Pending,
    Staged,
    Changes,
}

pub(super) fn resolve_path(
    repo: &RepoManager,
    repo_name: &str,
    scope: ScTargetScope,
    target: &ScPathTarget,
) -> Result<String> {
    let path = to_forward_slash(&target.path);
    if let Some(doc_id) = target.doc_id {
        let entries = repo.run_on_local_repo(repo_name, |db| entries_for_doc(db, scope, doc_id))?;
        if let Some(resolved) = resolve_from_entries(&entries, &path, Some(doc_id)) {
            return Ok(resolved);
        }
    }
    let entries = match scope {
        ScTargetScope::Pending => repo.list_pending_fs_in_local_repo(repo_name)?,
        ScTargetScope::Staged => repo.list_staged_in_local_repo(repo_name)?,
        ScTargetScope::Changes => repo.list_changes_in_local_repo(repo_name)?,
    };
    Ok(resolve_from_entries(&entries, &path, target.doc_id).unwrap_or(path))
}

fn entries_for_doc(db: &Database, scope: ScTargetScope, doc_id: DocId) -> Result<Vec<ChangeEntry>> {
    match scope {
        ScTargetScope::Pending => pending_entries(db, doc_id),
        ScTargetScope::Staged => staged_entries(db, doc_id),
        ScTargetScope::Changes => change_entries(db, doc_id),
    }
}

fn pending_entries(db: &Database, doc_id: DocId) -> Result<Vec<ChangeEntry>> {
    Ok(pending_fs::list_for_doc(db, doc_id)?
        .into_iter()
        .map(|entry| ChangeEntry {
            path: entry.path,
            renamed_from: entry.renamed_from,
            doc_id: entry.doc_id,
            status: entry.change_type,
            has_conflict: entry.has_conflict,
        })
        .collect())
}

fn staged_entries(db: &Database, doc_id: DocId) -> Result<Vec<ChangeEntry>> {
    Ok(staging::list_staged_entries_for_doc(db, doc_id)?
        .into_iter()
        .map(|(path, entry)| ChangeEntry {
            path,
            renamed_from: entry.renamed_from,
            doc_id: entry.doc_id,
            status: entry.status,
            has_conflict: entry.has_conflict,
        })
        .collect())
}

fn change_entries(db: &Database, doc_id: DocId) -> Result<Vec<ChangeEntry>> {
    let staged = staged_entries(db, doc_id)?;
    let staged_paths: HashSet<String> = staged
        .iter()
        .map(|entry| to_forward_slash(&entry.path))
        .collect();
    let mut changes = staged;
    changes.extend(
        pending_entries(db, doc_id)?
            .into_iter()
            .filter(|entry| !staged_paths.contains(&to_forward_slash(&entry.path))),
    );
    Ok(changes)
}

fn resolve_from_entries(
    entries: &[ChangeEntry],
    path: &str,
    doc_id: Option<DocId>,
) -> Option<String> {
    let by_doc = doc_id.and_then(|doc_id| {
        entries
            .iter()
            .find(|entry| entry.doc_id == Some(doc_id) && to_forward_slash(&entry.path) == path)
            .or_else(|| {
                entries.iter().find(|entry| {
                    entry.doc_id == Some(doc_id) && entry.status != ChangeStatus::Deleted
                })
            })
            .or_else(|| entries.iter().find(|entry| entry.doc_id == Some(doc_id)))
    });
    by_doc
        .or_else(|| {
            entries.iter().find(|entry| {
                to_forward_slash(&entry.path) == path && entry.status != ChangeStatus::Deleted
            })
        })
        .or_else(|| {
            entries.iter().find(|entry| {
                entry.status != ChangeStatus::Deleted
                    && entry
                        .renamed_from
                        .as_ref()
                        .is_some_and(|old_path| to_forward_slash(old_path) == path)
            })
        })
        .or_else(|| {
            entries
                .iter()
                .find(|entry| to_forward_slash(&entry.path) == path)
        })
        .map(|entry| to_forward_slash(&entry.path))
}

#[cfg(test)]
mod tests {
    use super::resolve_from_entries;
    use crate::models::DocId;
    use crate::source_control::{ChangeEntry, ChangeStatus};

    #[test]
    fn resolve_from_entries_matches_renamed_from_without_doc_id() {
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
            resolve_from_entries(&entries, "notes/old.md", None),
            Some("notes/new.md".into())
        );
    }

    #[test]
    fn resolve_from_entries_prefers_doc_id_when_available() {
        let doc_id = DocId(uuid::Uuid::nil());
        let entries = vec![ChangeEntry {
            path: "notes/new.md".into(),
            renamed_from: Some("notes/old.md".into()),
            doc_id: Some(doc_id),
            status: ChangeStatus::Added,
            has_conflict: false,
        }];

        assert_eq!(
            resolve_from_entries(&entries, "notes/old.md", Some(doc_id)),
            Some("notes/new.md".into())
        );
    }
}
