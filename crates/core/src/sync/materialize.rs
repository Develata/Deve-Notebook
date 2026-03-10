use super::persist_guard::PersistGuard;
use super::rebuild;
use crate::ledger::RepoManager;
use crate::ledger::listing::RepoListing;
use anyhow::Result;
use std::path::Path;

/// 启动时确保所有本地 repo 都拥有独立的 `vault/<repo_name>/` 工作区。
pub(super) fn prepare_local_workspaces(
    repo: &RepoManager,
    _vault_root: &Path,
    guard: &PersistGuard,
) -> Result<()> {
    for repo_name in repo.list_repos(None)? {
        materialize_local_repo(repo, guard, &repo_name)?;
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
        repo.bind_workspace_inode_in_local_repo(repo_name, &repo_path, doc_id)?;
    }

    Ok(())
}
