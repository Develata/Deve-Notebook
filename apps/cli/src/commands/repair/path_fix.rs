use anyhow::Result;
use deve_core::ledger::RepoManager;
use deve_core::source_control::pending_fs::{self, PendingFsEntry};
use std::path::PathBuf;
use std::sync::Arc;

pub(super) fn repair_repo_prefixed_paths(
    repo: &Arc<RepoManager>,
    repo_names: &[String],
) -> Result<usize> {
    let main_repo = repo.local_repo_name().to_string();
    let mut fixed = 0usize;
    for repo_name in repo_names {
        if repo_name == &main_repo {
            continue;
        }
        let docs = repo.list_local_docs(Some(repo_name))?;
        for (doc_id, old_path) in docs {
            let Some(stripped) = old_path
                .strip_prefix(repo_name.as_str())
                .and_then(|rest| rest.strip_prefix('/'))
            else {
                continue;
            };
            if stripped.is_empty() || path_exists_for_repair(repo, repo_name, stripped, doc_id)? {
                continue;
            }
            repo.repair_rename_doc_mapping_in_local_repo(repo_name, &old_path, stripped)?;
            rename_workspace_file(repo, repo_name, &old_path, stripped)?;
            move_pending(repo, repo_name, doc_id, &old_path, stripped)?;
            println!(
                "repair: normalized repo path {}:{} -> {}",
                repo_name, old_path, stripped
            );
            fixed += 1;
        }
    }
    Ok(fixed)
}

fn path_exists_for_repair(
    repo: &Arc<RepoManager>,
    repo_name: &str,
    path: &str,
    doc_id: deve_core::models::DocId,
) -> Result<bool> {
    Ok(repo
        .tracked_docid_or_legacy_error_in_local_repo(repo_name, path)?
        .is_some_and(|existing| existing != doc_id))
}

fn rename_workspace_file(
    repo: &RepoManager,
    repo_name: &str,
    old_path: &str,
    new_path: &str,
) -> Result<()> {
    let old_abs = repo.local_repo_workspace_path(repo_name, old_path)?;
    if !old_abs.exists() {
        return Ok(());
    }
    let new_abs = repo.local_repo_workspace_path(repo_name, new_path)?;
    if new_abs.exists() {
        return Ok(());
    }
    if let Some(parent) = new_abs.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::rename(&old_abs, &new_abs)?;
    prune_empty_parents(repo.local_repo_workspace_root(repo_name)?, old_abs.parent());
    Ok(())
}

fn move_pending(
    repo: &RepoManager,
    repo_name: &str,
    doc_id: deve_core::models::DocId,
    old_path: &str,
    new_path: &str,
) -> Result<()> {
    repo.run_on_local_repo(repo_name, |db| {
        let Some(entry) = pending_fs::list_for_doc(db, doc_id)?
            .into_iter()
            .find(|entry| entry.path == old_path)
        else {
            return Ok(());
        };
        pending_fs::remove(db, old_path)?;
        let moved = PendingFsEntry {
            path: new_path.to_string(),
            ..entry
        };
        pending_fs::upsert(db, &moved)
    })
}

fn prune_empty_parents(root: PathBuf, start: Option<&std::path::Path>) {
    let mut cursor = start.map(|p| p.to_path_buf());
    while let Some(path) = cursor {
        if path == root || !path.starts_with(&root) {
            break;
        }
        let is_empty = std::fs::read_dir(&path)
            .ok()
            .and_then(|mut iter| iter.next())
            .is_none();
        if !is_empty {
            break;
        }
        let parent = path.parent().map(|p| p.to_path_buf());
        let _ = std::fs::remove_dir(&path);
        cursor = parent;
    }
}

#[cfg(test)]
mod tests {
    use super::{move_pending, path_exists_for_repair};
    use deve_core::ledger::RepoManager;
    use deve_core::ledger::schema::{DOCID_TO_PATH, PATH_TO_DOCID};
    use deve_core::models::DocId;
    use deve_core::source_control::ChangeStatus;
    use deve_core::source_control::pending_fs::{self, PendingFsEntry};
    use std::sync::Arc;
    use tempfile::tempdir;

    #[test]
    fn path_exists_for_repair_prefers_tracked_lookup() -> anyhow::Result<()> {
        let dir = tempdir()?;
        let repo = Arc::new(RepoManager::init(
            dir.path(),
            10,
            Some("default"),
            Some("urn:default"),
        )?);
        let doc_id = DocId::new();
        assert!(!path_exists_for_repair(
            &repo,
            "default",
            "notes/a.md",
            doc_id
        )?);
        Ok(())
    }

    #[test]
    fn path_exists_for_repair_fails_closed_for_legacy_only_path_mapping() -> anyhow::Result<()> {
        let dir = tempdir()?;
        let repo = Arc::new(RepoManager::init(
            dir.path(),
            10,
            Some("default"),
            Some("urn:default"),
        )?);
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
            Ok(())
        })?;

        let err = path_exists_for_repair(&repo, "default", "notes/legacy.md", DocId::new())
            .expect_err("legacy-only repair collision check must fail closed");
        assert!(
            err.to_string()
                .contains("Tracked document projection missing for legacy-mapped path")
        );
        Ok(())
    }

    #[test]
    fn path_exists_for_repair_rejects_other_tracked_doc() -> anyhow::Result<()> {
        let dir = tempdir()?;
        let repo = Arc::new(RepoManager::init(
            dir.path(),
            10,
            Some("default"),
            Some("urn:default"),
        )?);
        let live = DocId::new();
        repo.apply_file_structure_in_local_repo("default", "notes/live.md", Some(live), "test")?;

        assert!(path_exists_for_repair(
            &repo,
            "default",
            "notes/live.md",
            DocId::new()
        )?);
        Ok(())
    }

    #[test]
    fn move_pending_ignores_entries_without_matching_doc_id() -> anyhow::Result<()> {
        let dir = tempdir()?;
        let repo = Arc::new(RepoManager::init(
            dir.path(),
            10,
            Some("default"),
            Some("urn:default"),
        )?);
        repo.run_on_local_repo("default", |db| {
            pending_fs::upsert(
                db,
                &PendingFsEntry {
                    path: "default/notes/live.md".into(),
                    renamed_from: None,
                    doc_id: None,
                    change_type: ChangeStatus::Modified,
                    content_hash: String::new(),
                    detected_at: 1,
                    has_conflict: false,
                },
            )
        })?;

        move_pending(
            &repo,
            "default",
            DocId::new(),
            "default/notes/live.md",
            "notes/live.md",
        )?;
        let old_exists = repo.run_on_local_repo("default", |db| {
            Ok(pending_fs::get(db, "default/notes/live.md")?.is_some())
        })?;
        assert!(old_exists);
        Ok(())
    }
}
