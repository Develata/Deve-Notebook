use crate::ledger::RepoManager;
use crate::ledger::traits::{RepoSelector, Repository};
use crate::protocol::ScPathTarget;
use crate::source_control::{ChangeEntry, ChangeStatus};
use crate::utils::path::to_forward_slash;
use rhai::EvalAltResult;

pub(super) fn resolve_local_sc_target(
    repo: &dyn Repository,
    repo_manager: &RepoManager,
    path: &str,
) -> Result<ScPathTarget, Box<EvalAltResult>> {
    let path = to_forward_slash(path);
    let selector = RepoSelector::default();
    let changes = repo
        .list_changes_in_repo(&selector)
        .map_err(|e| e.to_string())?;
    if let Some(entry) = resolve_entry(&changes, &path) {
        return Ok(ScPathTarget {
            path: to_forward_slash(&entry.path),
            doc_id: entry.doc_id,
        });
    }
    let doc_id = repo_manager
        .get_tracked_docid_in_local_repo(repo_manager.local_repo_name(), &path)
        .map_err(|e| e.to_string())?;
    Ok(ScPathTarget { path, doc_id })
}

fn resolve_entry<'a>(entries: &'a [ChangeEntry], path: &str) -> Option<&'a ChangeEntry> {
    let renamed_successor = entries.iter().find(|entry| {
        entry.status != ChangeStatus::Deleted
            && entry
                .renamed_from
                .as_ref()
                .is_some_and(|old_path| to_forward_slash(old_path) == path)
    });
    let path_reused_after_delete = renamed_successor.is_some()
        && entries.iter().any(|entry| {
            to_forward_slash(&entry.path) == path && entry.status == ChangeStatus::Deleted
        });
    if path_reused_after_delete {
        return renamed_successor;
    }
    entries
        .iter()
        .find(|entry| {
            to_forward_slash(&entry.path) == path && entry.status != ChangeStatus::Deleted
        })
        .or(renamed_successor)
        .or_else(|| entries.iter().find(|entry| to_forward_slash(&entry.path) == path))
}

#[cfg(test)]
mod tests {
    use super::resolve_entry;
    use crate::models::DocId;
    use crate::source_control::{ChangeEntry, ChangeStatus};

    #[test]
    fn resolve_entry_prefers_rename_successor_over_reused_old_path() {
        let doc_id = DocId::new();
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
            ChangeEntry {
                path: "notes/old.md".into(),
                renamed_from: None,
                doc_id: None,
                status: ChangeStatus::Added,
                has_conflict: false,
            },
        ];

        let resolved = resolve_entry(&entries, "notes/old.md").expect("resolved target");
        assert_eq!(resolved.path, "notes/new.md");
        assert_eq!(resolved.doc_id, Some(doc_id));
    }
}
