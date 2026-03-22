use anyhow::{Context, Result};
use deve_core::ledger::RepoManager;
use deve_core::models::Op;
use deve_core::sync::{SyncManager, reconcile};
use deve_core::utils::path::to_forward_slash;
use std::path::{Path, PathBuf};
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
        paths
            .iter()
            .map(|path| normalize_restore_path(repo_name, path))
            .collect()
    };
    let mut restored = 0usize;
    for repo_path in targets {
        let Some(doc_id) = resolve_repair_docid(repo, repo_name, &repo_path)? else {
            println!("repair: skip {}:{}, doc not found", repo_name, repo_path);
            continue;
        };
        let Some(backup_path) = resolve_backup_path(backup_root, repo_name, &repo_path)? else {
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
    let mut targets = Vec::new();
    for (_, repo_path) in repo.list_local_docs(Some(repo_name))? {
        if workspace_starts_with_loading(repo, repo_name, &repo_path)?
            && resolve_repair_docid(repo, repo_name, &repo_path)?.is_some()
        {
            targets.push(repo_path);
        }
    }
    Ok(targets)
}

fn workspace_starts_with_loading(
    repo: &Arc<RepoManager>,
    repo_name: &str,
    repo_path: &str,
) -> Result<bool> {
    let path = repo.local_repo_workspace_path(repo_name, repo_path)?;
    match std::fs::read_to_string(path) {
        Ok(current) => Ok(current.starts_with("# Loading...")),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(err) => Err(err.into()),
    }
}

fn resolve_repair_docid(
    repo: &Arc<RepoManager>,
    repo_name: &str,
    repo_path: &str,
) -> Result<Option<deve_core::models::DocId>> {
    repo.get_tracked_docid_in_local_repo(repo_name, repo_path)
}

fn normalize_restore_path(repo_name: &str, path: &str) -> String {
    let path = to_forward_slash(path);
    path.strip_prefix(repo_name)
        .and_then(|rest| rest.strip_prefix('/'))
        .filter(|rest| !rest.is_empty())
        .map(str::to_string)
        .unwrap_or(path)
}

fn resolve_backup_path(
    backup_root: &Path,
    repo_name: &str,
    repo_path: &str,
) -> Result<Option<PathBuf>> {
    let prefixed = backup_root.join(repo_name).join(repo_path);
    match prefixed.try_exists() {
        Ok(true) => Ok(Some(prefixed)),
        Ok(false) => Ok(None),
        Err(err) => Err(err).context(format!(
            "Failed to stat backup path for {}:{}",
            repo_name, repo_path
        )),
    }
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
#[path = "restore_test.rs"]
mod tests;
