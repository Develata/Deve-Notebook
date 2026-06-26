//! plan_ref:
//!   - 03_storage/index#internal-path-normalization
//!   - 04_repository#tree-projection-contract
//!   - 14_commands#cli-commands

use anyhow::Result;
use deve_core::ledger::RepoManager;
use deve_core::source_control::pending_fs;
use deve_core::utils::fs::checked_exists;
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
        if !entry.file_type()?.is_dir() {
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
    let dst = move_tree_to_quarantine(repo, repo_name, rel, dir)?;
    if let Err(err) = remove_ledger_subtree(repo, repo_name, rel) {
        rollback_quarantined_tree(&dst, dir)?;
        return Err(err);
    }
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
        let _ = pending_fs::remove_subtree(db, &prefix)?;
        Ok(())
    })
}

fn move_tree_to_quarantine(
    repo: &Arc<RepoManager>,
    repo_name: &str,
    rel: &Path,
    dir: &Path,
) -> Result<PathBuf> {
    let quarantine_root = repo
        .local_repo_notegit_root(repo_name)?
        .join("legacy-md-dir");
    let dst = unique_path(quarantine_root.join(rel))?;
    if let Some(parent) = dst.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::rename(dir, &dst)?;
    Ok(dst)
}

fn rollback_quarantined_tree(quarantine_path: &Path, original_path: &Path) -> Result<()> {
    if !checked_exists(quarantine_path, "quarantine rollback source")? {
        anyhow::bail!(
            "Quarantined tree rollback source missing: {:?}",
            quarantine_path
        );
    }
    if checked_exists(original_path, "quarantine rollback target")? {
        anyhow::bail!(
            "Quarantined tree rollback target already exists: {:?}",
            original_path
        );
    }
    if let Some(parent) = original_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::rename(quarantine_path, original_path)?;
    Ok(())
}

fn unique_path(path: PathBuf) -> Result<PathBuf> {
    if !checked_exists(&path, "quarantine target path")? {
        return Ok(path);
    }
    for idx in 1.. {
        let candidate = path.with_extension(format!(
            "{}.{}",
            path.extension()
                .and_then(|ext| ext.to_str())
                .unwrap_or("bak"),
            idx
        ));
        if !checked_exists(&candidate, "quarantine candidate path")? {
            return Ok(candidate);
        }
    }
    // 不可达：`1..` 在 2^63 个候选耗尽前必有一个未占用路径触发上面的 return；
    // 文件系统不可能容纳那么多同名候选，循环必然提前返回。
    unreachable!()
}

#[cfg(test)]
mod tests;
