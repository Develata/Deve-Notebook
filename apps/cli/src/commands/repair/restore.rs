use anyhow::Result;
use deve_core::ledger::RepoManager;
use deve_core::models::Op;
use deve_core::sync::{SyncManager, reconcile};
use deve_core::utils::notegit::is_internal_repo_path;
use deve_core::utils::path::{path_to_forward_slash, to_forward_slash};
use std::path::Path;
use std::sync::Arc;

pub(super) fn restore_docs_from_backup(
    repo: &Arc<RepoManager>,
    sync_manager: &SyncManager,
    backup_root: &Path,
    repo_names: &[String],
    paths: &[String],
) -> Result<usize> {
    let mut restored = 0usize;
    for repo_name in repo_names {
        restored += restore_repo(repo, sync_manager, backup_root, repo_name, paths)?;
    }
    Ok(restored)
}

fn restore_repo(
    repo: &Arc<RepoManager>,
    sync_manager: &SyncManager,
    backup_root: &Path,
    repo_name: &str,
    paths: &[String],
) -> Result<usize> {
    let targets = if paths.is_empty() {
        find_loading_corruption(repo, repo_name)?
    } else {
        paths.iter().map(|path| to_forward_slash(path)).collect()
    };
    let mut restored = 0usize;
    for repo_path in targets {
        let Some(doc_id) = resolve_repair_docid(repo, repo_name, &repo_path)? else {
            println!("repair: skip {}:{}, doc not found", repo_name, repo_path);
            continue;
        };
        let Some(backup_path) = resolve_backup_path(backup_root, repo_name, &repo_path) else {
            println!("repair: skip {}:{}, backup missing", repo_name, repo_path);
            continue;
        };
        let target = std::fs::read_to_string(&backup_path)?;
        let entries: Vec<_> = repo
            .get_local_ops_in_local_repo(repo_name, doc_id)?
            .into_iter()
            .map(|(_, entry)| entry)
            .collect();
        let current = deve_core::state::reconstruct_content(&entries);
        let patch = overwrite_patch(&current, &target);
        if patch.is_empty() {
            continue;
        }
        reconcile::append_patch_in_local_repo(repo.as_ref(), repo_name, doc_id, "repair", &patch)?;
        sync_manager.persist_doc_in_local_repo(repo_name, doc_id)?;
        println!(
            "repair: restored {}:{} from {:?}",
            repo_name, repo_path, backup_path
        );
        restored += 1;
    }
    Ok(restored)
}

fn find_loading_corruption(repo: &Arc<RepoManager>, repo_name: &str) -> Result<Vec<String>> {
    let root = repo.local_repo_workspace_root(repo_name)?;
    let mut targets = Vec::new();
    walk_repo(repo, repo_name, &root, &root, &mut targets)?;
    Ok(targets)
}

fn walk_repo(
    repo: &Arc<RepoManager>,
    repo_name: &str,
    root: &Path,
    dir: &Path,
    targets: &mut Vec<String>,
) -> Result<()> {
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            walk_repo(repo, repo_name, root, &path, targets)?;
            continue;
        }
        if path.extension().and_then(|ext| ext.to_str()) != Some("md") {
            continue;
        }
        let rel = path.strip_prefix(root)?;
        let repo_path = path_to_forward_slash(rel);
        if is_internal_repo_path(&repo_path) {
            continue;
        }
        let current = std::fs::read_to_string(&path).unwrap_or_default();
        if current.starts_with("# Loading...")
            && resolve_repair_docid(repo, repo_name, &repo_path)?.is_some()
        {
            targets.push(repo_path);
        }
    }
    Ok(())
}

fn resolve_repair_docid(
    repo: &Arc<RepoManager>,
    repo_name: &str,
    repo_path: &str,
) -> Result<Option<deve_core::models::DocId>> {
    repo.get_tracked_docid_in_local_repo(repo_name, repo_path)
}

fn resolve_backup_path(
    backup_root: &Path,
    repo_name: &str,
    repo_path: &str,
) -> Option<std::path::PathBuf> {
    let prefixed = backup_root.join(repo_name).join(repo_path);
    prefixed.exists().then_some(prefixed)
}

fn overwrite_patch(current: &str, target: &str) -> Vec<Op> {
    if current == target {
        return Vec::new();
    }
    let mut ops = Vec::with_capacity(2);
    let current_len = current.encode_utf16().count() as u32;
    if current_len > 0 {
        ops.push(Op::Delete {
            pos: 0,
            len: current_len,
        });
    }
    if !target.is_empty() {
        ops.push(Op::Insert {
            pos: 0,
            content: target.into(),
        });
    }
    ops
}

#[cfg(test)]
mod tests {
    use super::{resolve_backup_path, resolve_repair_docid};
    use deve_core::ledger::RepoManager;
    use deve_core::ledger::schema::{DOCID_TO_PATH, PATH_TO_DOCID};
    use deve_core::models::DocId;
    use std::sync::Arc;
    use tempfile::tempdir;

    #[test]
    fn resolve_repair_docid_returns_tracked_docid() -> anyhow::Result<()> {
        let dir = tempdir()?;
        let repo = Arc::new(RepoManager::init(
            dir.path(),
            10,
            Some("default"),
            Some("urn:default"),
        )?);
        let doc_id = DocId::new();
        repo.apply_file_structure_in_local_repo("default", "notes/live.md", Some(doc_id), "test")?;
        assert_eq!(
            resolve_repair_docid(&repo, "default", "notes/live.md")?,
            Some(doc_id)
        );
        Ok(())
    }

    #[test]
    fn resolve_repair_docid_ignores_legacy_only_path_mapping() -> anyhow::Result<()> {
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

        assert_eq!(
            resolve_repair_docid(&repo, "default", "notes/legacy.md")?,
            None
        );
        Ok(())
    }

    #[test]
    fn resolve_backup_path_requires_repo_scoped_backup_layout() -> anyhow::Result<()> {
        let dir = tempdir()?;
        let direct = dir.path().join("notes/live.md");
        std::fs::create_dir_all(direct.parent().expect("parent"))?;
        std::fs::write(&direct, "backup")?;

        assert_eq!(resolve_backup_path(dir.path(), "default", "notes/live.md"), None);
        Ok(())
    }
}
