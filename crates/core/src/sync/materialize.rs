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
    for repo_name in repo.list_repos(None)? {
        materialize_local_repo(repo, vault_root, guard, &repo_name)?;
    }
    archive_legacy_flat_entries(repo, vault_root)
}

/// 将指定本地 repo 的文档视图投影到 `vault/<repo_name>/`。
///
/// Invariants:
/// - 仅在 repo 工作区首次缺失时执行 bootstrap；已有工作区绝不覆盖用户文件。
/// - 不会删除其他 repo 的工作区内容，也不会吞掉未入库的 working tree 变更。
pub(super) fn materialize_local_repo(
    repo: &RepoManager,
    _vault_root: &Path,
    guard: &PersistGuard,
    repo_name: &str,
) -> Result<()> {
    let repo_root = repo.local_repo_workspace_root(repo_name)?;
    let bootstrap = !repo_root.exists();
    std::fs::create_dir_all(&repo_root)?;
    if !bootstrap {
        return Ok(());
    }

    for (doc_id, repo_path) in repo.list_local_docs(Some(repo_name))? {
        let file_path = repo.local_repo_workspace_path(repo_name, &repo_path)?;
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

fn archive_legacy_flat_entries(repo: &RepoManager, vault_root: &Path) -> Result<()> {
    let repo_names: HashSet<String> = repo.list_repos(None)?.into_iter().collect();
    let archive_root = vault_root.join(".deve").join("legacy-flat");
    std::fs::create_dir_all(&archive_root)?;

    for entry in std::fs::read_dir(vault_root)? {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with('.') || repo_names.contains(&name) {
            continue;
        }
        let src = entry.path();
        let dst = unique_archive_path(&archive_root, &name);
        if let Err(err) = std::fs::rename(&src, &dst) {
            tracing::warn!(
                "Legacy workspace archive failed {:?} -> {:?}: {:?}",
                src,
                dst,
                err
            );
        }
    }
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
