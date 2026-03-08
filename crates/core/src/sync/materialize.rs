use super::persist_guard::PersistGuard;
use super::rebuild;
use crate::ledger::RepoManager;
use crate::ledger::listing::RepoListing;
use anyhow::Result;
use std::collections::HashSet;
use std::path::{Path, PathBuf};

/// 启动时确保所有本地 repo 都拥有独立的 `vault/<repo_name>/` 工作区。
pub(super) fn prepare_local_workspaces(
    repo: &RepoManager,
    vault_root: &Path,
    guard: &PersistGuard,
) -> Result<()> {
    migrate_legacy_flat_entries(repo, vault_root)?;
    for repo_name in repo.list_repos(None)? {
        materialize_local_repo(repo, vault_root, guard, &repo_name)?;
    }
    Ok(())
}

/// 将指定本地 repo 的文档视图投影到 `vault/<repo_name>/`。
///
/// Invariants:
/// - 仅补齐缺失文件；已有工作区绝不覆盖用户文件。
/// - 不会删除其他 repo 的工作区内容，也不会吞掉未入库的 working tree 变更。
pub(super) fn materialize_local_repo(
    repo: &RepoManager,
    _vault_root: &Path,
    guard: &PersistGuard,
    repo_name: &str,
) -> Result<()> {
    let repo_root = repo.local_repo_workspace_root(repo_name)?;
    std::fs::create_dir_all(&repo_root)?;
    std::fs::create_dir_all(repo.local_repo_notegit_root(repo_name)?)?;

    for (doc_id, repo_path) in repo.list_local_docs(Some(repo_name))? {
        if crate::utils::notegit::is_internal_repo_path(&repo_path) {
            tracing::warn!(
                "Skip materializing internal repo metadata path: {}/{}",
                repo_name,
                repo_path
            );
            continue;
        }
        let file_path = repo.local_repo_workspace_path(repo_name, &repo_path)?;
        if file_path.exists() {
            continue;
        }
        let rebuilt = rebuild::rebuild_local_doc_in_repo(repo, repo_name, doc_id)?;
        if let Some(parent) = file_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        guard.record(
            &repo.local_repo_workspace_relative(repo_name, &repo_path),
            &rebuilt.content,
        );
        std::fs::write(&file_path, rebuilt.content)?;
    }

    Ok(())
}

fn migrate_legacy_flat_entries(repo: &RepoManager, vault_root: &Path) -> Result<()> {
    let repo_names: HashSet<String> = repo.list_repos(None)?.into_iter().collect();
    let default_root = repo.local_repo_workspace_root(repo.local_repo_name())?;
    std::fs::create_dir_all(&default_root)?;
    let conflict_root = crate::utils::notegit::repo_legacy_flat_conflicts_dir(&default_root);
    std::fs::create_dir_all(&conflict_root)?;

    for entry in std::fs::read_dir(vault_root)? {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with('.') || repo_names.contains(&name) {
            continue;
        }
        let src = entry.path();
        let dst = default_root.join(&name);
        move_legacy_entry(&src, &dst, &conflict_root)?;
    }
    Ok(())
}

fn move_legacy_entry(src: &Path, dst: &Path, conflict_root: &Path) -> Result<()> {
    if !dst.exists() {
        if let Some(parent) = dst.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::rename(src, dst)?;
        return Ok(());
    }

    if src.is_dir() && dst.is_dir() {
        for entry in std::fs::read_dir(src)? {
            let entry = entry?;
            let child_src = entry.path();
            let child_dst = dst.join(entry.file_name());
            move_legacy_entry(&child_src, &child_dst, conflict_root)?;
        }
        let _ = std::fs::remove_dir(src);
        return Ok(());
    }

    if src.is_file() && dst.is_file() && std::fs::read(src)? == std::fs::read(dst)? {
        std::fs::remove_file(src)?;
        return Ok(());
    }

    let name = src
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("legacy");
    let archive = unique_archive_path(conflict_root, name);
    std::fs::rename(src, &archive)?;
    tracing::warn!(
        "Legacy flat workspace conflict archived {:?} -> {:?}",
        src,
        archive
    );
    Ok(())
}

fn unique_archive_path(root: &Path, name: &str) -> PathBuf {
    let first = root.join(name);
    if !first.exists() {
        return first;
    }
    for idx in 1.. {
        let candidate = root.join(format!("{name}-{idx}"));
        if !candidate.exists() {
            return candidate;
        }
    }
    unreachable!("archive suffix search is unbounded");
}
