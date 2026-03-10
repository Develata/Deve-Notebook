use anyhow::Result;
use deve_core::ledger::RepoManager;
use deve_core::source_control::pending_fs;
use deve_core::utils::notegit::is_internal_repo_path;
use deve_core::utils::path::path_to_forward_slash;
use std::path::{Path, PathBuf};
use std::sync::Arc;

pub(super) fn quarantine_md_dirs(repo: &Arc<RepoManager>, repo_names: &[String]) -> Result<usize> {
    let mut moved = 0usize;
    for repo_name in repo_names {
        let root = repo.local_repo_workspace_root(repo_name)?;
        walk(repo, repo_name, &root, &root, &mut moved)?;
    }
    Ok(moved)
}

fn walk(
    repo: &Arc<RepoManager>,
    repo_name: &str,
    root: &Path,
    dir: &Path,
    moved: &mut usize,
) -> Result<()> {
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let rel = path.strip_prefix(root)?;
        let repo_path = path_to_forward_slash(rel);
        if is_internal_repo_path(&repo_path) {
            continue;
        }
        if repo_path.split('/').any(|segment| segment.ends_with(".md")) {
            *moved += quarantine_tree(repo, repo_name, root, &path)?;
            continue;
        }
        walk(repo, repo_name, root, &path, moved)?;
    }
    Ok(())
}

fn quarantine_tree(
    repo: &Arc<RepoManager>,
    repo_name: &str,
    root: &Path,
    dir: &Path,
) -> Result<usize> {
    let rel = dir.strip_prefix(root)?;
    let quarantine_root = repo
        .local_repo_notegit_root(repo_name)?
        .join("legacy-md-dir");
    let dst = unique_path(quarantine_root.join(rel));
    if let Some(parent) = dst.parent() {
        std::fs::create_dir_all(parent)?;
    }
    remove_ledger_subtree(repo, repo_name, rel)?;
    std::fs::rename(dir, &dst)?;
    println!(
        "repair: quarantined invalid md-dir {}:{} -> {:?}",
        repo_name,
        path_to_forward_slash(rel),
        dst
    );
    Ok(1)
}

fn remove_ledger_subtree(repo: &Arc<RepoManager>, repo_name: &str, rel: &Path) -> Result<()> {
    let prefix = path_to_forward_slash(rel);
    repo.repair_delete_folder_mapping_in_local_repo(repo_name, &prefix)?;
    repo.run_on_local_repo(repo_name, |db| {
        let entries = pending_fs::list_all(db)?;
        for entry in entries {
            if entry.path == prefix || entry.path.starts_with(&(prefix.clone() + "/")) {
                pending_fs::remove(db, &entry.path)?;
            }
        }
        Ok(())
    })
}

fn unique_path(path: PathBuf) -> PathBuf {
    if !path.exists() {
        return path;
    }
    for idx in 1.. {
        let candidate = path.with_extension(format!(
            "{}.{}",
            path.extension()
                .and_then(|ext| ext.to_str())
                .unwrap_or("bak"),
            idx
        ));
        if !candidate.exists() {
            return candidate;
        }
    }
    unreachable!()
}
