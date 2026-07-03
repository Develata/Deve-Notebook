//! plan_ref:
//!   - 04_repository#tree-projection-contract
//!

use super::projection_plan;
use super::rebuild;
use crate::ledger::RepoManager;
use crate::ledger::inode_index;
use crate::utils::fs::checked_exists;
use crate::writeback::PersistGuard;
use anyhow::Result;
use std::collections::HashSet;
use tracing::warn;

/// 启动时确保所有已绑定 locator 的本地 repo 都拥有独立的 Projection Workspace。
pub(super) fn prepare_local_workspaces(
    repo: &RepoManager,
    guard: &PersistGuard,
    skip_repo_names: &HashSet<String>,
) -> Result<Vec<String>> {
    let mut skipped = Vec::new();
    for repo_name in repo.list_local_repo_names_for_execution()? {
        if skip_repo_names.contains(&repo_name) {
            skipped.push(repo_name);
            continue;
        }
        if let Err(err) = prepare_local_repo_for_startup(repo, guard, &repo_name) {
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

fn prepare_local_repo_for_startup(
    repo: &RepoManager,
    guard: &PersistGuard,
    repo_name: &str,
) -> Result<()> {
    // Startup scan must not erase external deletions that are already known to
    // belong to this workspace; explicit materialize/rebuild still restores all
    // missing files through materialize_local_repo.
    let bound_doc_ids = repo.run_on_local_repo(repo_name, inode_index::list_docids)?;
    materialize_local_repo_with_policy(repo, guard, repo_name, Some(&bound_doc_ids))
}

/// 将指定本地 repo 的文档视图投影到 `<projection_base>/<safe_repo_name>--<repo_id>/`。
///
/// Invariants:
/// - 仅补齐缺失文件；已有工作区绝不覆盖用户文件。
/// - 不会删除其他 repo 的工作区内容，也不会吞掉未入库的 working tree 变更。
pub(super) fn materialize_local_repo(
    repo: &RepoManager,
    guard: &PersistGuard,
    repo_name: &str,
) -> Result<()> {
    materialize_local_repo_with_policy(repo, guard, repo_name, None)
}

fn materialize_local_repo_with_policy(
    repo: &RepoManager,
    guard: &PersistGuard,
    repo_name: &str,
    skip_missing_bound_doc_ids: Option<&HashSet<crate::models::DocId>>,
) -> Result<()> {
    let repo_root = repo.local_repo_workspace_root(repo_name)?;
    std::fs::create_dir_all(&repo_root)?;
    repo.ensure_local_repo_workspace_identity(repo_name)?;
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
        if skip_missing_bound_doc_ids.is_some_and(|doc_ids| doc_ids.contains(&doc_id)) {
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
