//! plan_ref:
//!   - 04_repository#tree-projection-contract
//!

use super::projection_plan;
use super::rebuild;
use crate::ledger::RepoManager;
use crate::ledger::inode_index;
use crate::ledger::range;
use crate::models::{DocId, RepoId};
use crate::utils::fs::checked_exists;
use crate::writeback::PersistGuard;
use anyhow::Result;
use std::collections::HashSet;
use std::path::PathBuf;
use tracing::warn;

/// Lock-free projection preparation captured from one stable ledger head.
/// Filesystem writes and inode binding are deferred to `apply` under the
/// repository mutation permit.
pub struct PreparedLocalRepoMaterialization {
    repo_id: RepoId,
    repo_name: String,
    expected_ledger_head: u64,
    repo_root: PathBuf,
    directories: Vec<String>,
    missing_docs: Vec<PreparedMissingDoc>,
}

struct PreparedMissingDoc {
    repo_path: String,
    doc_id: DocId,
    content: String,
}

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
    let prepared = prepare_local_repo_materialization(repo, repo_name)?;
    apply_prepared_local_repo_materialization(repo, guard, prepared)
}

pub(super) fn prepare_local_repo_materialization(
    repo: &RepoManager,
    repo_name: &str,
) -> Result<PreparedLocalRepoMaterialization> {
    let info = repo
        .get_repo_info_for(None, Some(repo_name))?
        .ok_or_else(|| anyhow::anyhow!("Repository not found: {repo_name}"))?;
    let repo_name = repo.resolve_local_repo_name_for_execution(Some(info.uuid), Some(repo_name))?;
    let expected_ledger_head = repo.run_on_local_repo(&repo_name, range::get_max_seq)?;
    let repo_root = repo.local_repo_workspace_root(&repo_name)?;
    let plan = projection_plan::build(repo, &repo_name)?;
    let mut directories = plan.dirs.into_iter().collect::<Vec<_>>();
    directories.sort_by_key(|path| (path.matches('/').count(), path.clone()));
    let mut docs = plan.docs.into_iter().collect::<Vec<_>>();
    docs.sort_by(|left, right| left.0.cmp(&right.0));
    let mut missing_docs = Vec::new();
    for (repo_path, doc_id) in docs {
        let file_path = repo.local_repo_workspace_path(&repo_name, &repo_path)?;
        if checked_exists(&file_path, "workspace path while preparing materialization")? {
            continue;
        }
        let rebuilt = rebuild::rebuild_local_doc_in_repo(repo, &repo_name, doc_id)?;
        missing_docs.push(PreparedMissingDoc {
            repo_path,
            doc_id,
            content: rebuilt.content,
        });
    }
    let observed_head = repo.run_on_local_repo(&repo_name, range::get_max_seq)?;
    if observed_head != expected_ledger_head {
        anyhow::bail!(
            "projection materialization source changed during preparation: expected head {}, observed {}",
            expected_ledger_head,
            observed_head
        );
    }
    Ok(PreparedLocalRepoMaterialization {
        repo_id: info.uuid,
        repo_name,
        expected_ledger_head,
        repo_root,
        directories,
        missing_docs,
    })
}

pub(super) fn apply_prepared_local_repo_materialization(
    repo: &RepoManager,
    guard: &PersistGuard,
    prepared: PreparedLocalRepoMaterialization,
) -> Result<()> {
    let bound_name = repo
        .resolve_local_repo_name_for_execution(Some(prepared.repo_id), Some(&prepared.repo_name))?;
    let observed_head = repo.run_on_local_repo(&bound_name, range::get_max_seq)?;
    if observed_head != prepared.expected_ledger_head {
        anyhow::bail!(
            "projection materialization source changed before apply: expected head {}, observed {}",
            prepared.expected_ledger_head,
            observed_head
        );
    }
    std::fs::create_dir_all(&prepared.repo_root)?;
    repo.ensure_local_repo_workspace_identity(&bound_name)?;
    crate::utils::notegit::ensure_gitignore_ignores_notegit(&prepared.repo_root)?;
    for directory in prepared.directories {
        if directory.is_empty() {
            continue;
        }
        std::fs::create_dir_all(repo.local_repo_workspace_path(&bound_name, &directory)?)?;
    }
    for doc in prepared.missing_docs {
        let file_path = repo.local_repo_workspace_path(&bound_name, &doc.repo_path)?;
        if checked_exists(&file_path, "workspace path while applying materialization")? {
            continue;
        }
        if let Some(parent) = file_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let relative = repo.local_repo_workspace_relative(&bound_name, &doc.repo_path);
        guard.record(&relative, &doc.content);
        if let Err(error) = std::fs::write(&file_path, doc.content) {
            guard.clear(&relative);
            return Err(error.into());
        }
        repo.bind_workspace_inode_in_local_repo(&bound_name, &doc.repo_path, doc.doc_id)?;
    }
    Ok(())
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

#[cfg(test)]
mod tests {
    use super::{apply_prepared_local_repo_materialization, prepare_local_repo_materialization};
    use crate::ledger::RepoManager;
    use crate::utils::fs::checked_exists;
    use crate::writeback::PersistGuard;

    #[test]
    fn prepared_materialization_rejects_ledger_head_drift_before_filesystem_writes()
    -> anyhow::Result<()> {
        let dir = tempfile::tempdir()?;
        let projection_base = dir.path().join("projection");
        let mut repo = RepoManager::init(dir.path().join("ledger"), 10, None, None)?;
        repo.set_projection_base_for_all_local_repos_checked(&projection_base)?;
        let repo_name = repo.local_repo_name().to_string();
        repo.apply_file_structure_in_local_repo(&repo_name, "first.md", None, "test")?;

        let prepared = prepare_local_repo_materialization(&repo, &repo_name)?;
        let repo_root = repo.local_repo_workspace_root(&repo_name)?;
        assert!(!checked_exists(&repo_root, "materialization test root")?);

        repo.apply_file_structure_in_local_repo(&repo_name, "second.md", None, "test")?;
        let error =
            apply_prepared_local_repo_materialization(&repo, &PersistGuard::new(), prepared)
                .expect_err("stale materialization must fail closed");

        assert!(error.to_string().contains("source changed before apply"));
        assert!(
            !checked_exists(&repo_root, "materialization test root after rejection")?,
            "stale plan must be rejected before creating the workspace"
        );
        Ok(())
    }
}
