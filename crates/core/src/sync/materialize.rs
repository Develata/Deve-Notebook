//! plan_ref:
//!   - 06_repository#tree-projection-contract
//!

use super::projection_plan;
use super::rebuild;
use crate::ledger::RepoManager;
use crate::utils::fs::checked_exists;
use crate::writeback::PersistGuard;
use anyhow::Result;
use std::path::Path;
use tracing::warn;

/// 启动时确保所有本地 repo 都拥有独立的 `vault/<repo_name>/` 工作区。
pub(super) fn prepare_local_workspaces(
    repo: &RepoManager,
    _vault_root: &Path,
    guard: &PersistGuard,
) -> Result<Vec<String>> {
    let mut skipped = Vec::new();
    for repo_name in repo.list_local_repo_names_for_execution()? {
        if let Err(err) = materialize_local_repo(repo, guard, &repo_name) {
            if is_broken_structure_projection_error(&err) {
                warn!(
                    repo_name = %repo_name,
                    error = %err,
                    "Sync startup skipped local repo with broken structure projection"
                );
                skipped.push(repo_name);
                continue;
            }
            return Err(err);
        }
    }
    Ok(skipped)
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
    crate::utils::notegit::ensure_gitignore_ignores_notegit(&repo_root)?;
    let plan = projection_plan::build(repo, repo_name)?;
    for dir in plan.dirs {
        if dir.is_empty() {
            continue;
        }
        std::fs::create_dir_all(repo.local_repo_workspace_path(repo_name, &dir)?)?;
    }

    for (repo_path, doc_id) in plan.docs {
        let file_path = repo.local_repo_workspace_path(repo_name, &repo_path)?;
        if checked_exists(&file_path, "workspace path while materializing projection")? {
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
        if let Err(err) = std::fs::write(&file_path, rebuilt.content) {
            guard.clear(&repo.local_repo_workspace_relative(repo_name, &repo_path));
            return Err(err.into());
        }
        repo.bind_workspace_inode_in_local_repo(repo_name, &repo_path, doc_id)?;
    }

    Ok(())
}

pub(super) fn is_broken_structure_projection_error(err: &anyhow::Error) -> bool {
    let lower = err.to_string().to_ascii_lowercase();
    lower.contains("structure projection references missing parent")
        || lower.contains("structure projection rename references missing node")
        || lower.contains("structure projection move references missing node")
        || lower.contains("structure projection contains cycle")
        || lower.contains("structure projection lost doc identity")
        || lower.contains("structure projection file node/doc mismatch")
        || lower.contains("structure projection parent is not a directory")
        || lower.contains("structure projection duplicate create")
        || lower.contains("structure projection path collision")
}
