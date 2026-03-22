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
        return target_for_resolved_path(repo_manager, &entry.path, entry.doc_id);
    }
    target_for_resolved_path(repo_manager, &path, None)
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
        .or_else(|| {
            entries
                .iter()
                .find(|entry| to_forward_slash(&entry.path) == path)
        })
}

fn target_for_resolved_path(
    repo_manager: &RepoManager,
    path: &str,
    fallback_doc_id: Option<crate::models::DocId>,
) -> Result<ScPathTarget, Box<EvalAltResult>> {
    let path = to_forward_slash(path);
    let doc_id = match fallback_doc_id {
        Some(doc_id) => Some(doc_id),
        None => repo_manager
            .get_tracked_docid_in_local_repo(repo_manager.local_repo_name(), &path)
            .map_err(|e| e.to_string())?,
    };
    Ok(ScPathTarget { path, doc_id })
}

#[cfg(test)]
mod tests {
    use super::{resolve_entry, resolve_local_sc_target};
    use crate::ledger::schema::{DOCID_TO_PATH, PATH_TO_DOCID};
    use crate::ledger::{RepoManager, node_meta};
    use crate::models::DocId;
    use crate::source_control::{ChangeEntry, ChangeStatus, pending_fs};
    use tempfile::tempdir;

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

    #[test]
    fn resolve_local_sc_target_returns_none_for_legacy_only_path_mapping() -> anyhow::Result<()> {
        let dir = tempdir()?;
        let mut repo = RepoManager::init(dir.path(), 10, None, None)?;
        repo.set_vault_root(dir.path().join("vault"));
        let doc_id = DocId::new();
        repo.run_on_local_repo("default", |db| {
            let write = db.begin_write()?;
            {
                let mut p2d = write.open_table(PATH_TO_DOCID)?;
                let mut d2p = write.open_table(DOCID_TO_PATH)?;
                p2d.insert("notes/legacy.md", doc_id.as_u128())?;
                d2p.insert(doc_id.as_u128(), "notes/legacy.md")?;
            }
            write.commit()?;
            Ok::<_, anyhow::Error>(())
        })?;

        let target = resolve_local_sc_target(&repo, &repo, "notes/legacy.md")
            .expect("legacy-only path returns Ok with no doc_id");

        assert_eq!(target.path, "notes/legacy.md");
        assert_eq!(
            target.doc_id, None,
            "legacy-only mapping must not resolve to doc_id"
        );
        Ok(())
    }

    #[test]
    fn resolve_local_sc_target_fills_doc_id_from_tracked_projection() -> anyhow::Result<()> {
        let dir = tempdir()?;
        let mut repo = RepoManager::init(dir.path(), 10, None, None)?;
        repo.set_vault_root(dir.path().join("vault"));
        let doc_id = DocId::new();
        repo.run_on_local_repo("default", |db| {
            node_meta::ensure_file_node(db, "notes/a.md", doc_id)?;
            pending_fs::upsert(
                db,
                &pending_fs::PendingFsEntry {
                    path: "notes/a.md".into(),
                    renamed_from: None,
                    doc_id: None,
                    change_type: ChangeStatus::Modified,
                    content_hash: "hash".into(),
                    detected_at: 0,
                    has_conflict: false,
                },
            )?;
            Ok::<_, anyhow::Error>(())
        })?;

        let target = resolve_local_sc_target(&repo, &repo, "notes/a.md").expect("resolved");
        assert_eq!(target.path, "notes/a.md");
        assert_eq!(target.doc_id, Some(doc_id));
        Ok(())
    }
}
