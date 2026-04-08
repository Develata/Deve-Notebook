use super::persist_guard::PersistGuard;
use super::projection_plan;
use super::rebuild;
use super::rebuild_projection_state;
use crate::ledger::RepoManager;
use anyhow::Result;
use std::collections::{HashMap, HashSet};
use std::path::Path;

/// 强制从 Ledger projection 重建本地 repo 工作区。
///
/// Invariants:
/// - 仅 Ledger projection 可决定受管 `.md` 的规范存在性与内容。
/// - `.notegit/` 与非 Markdown 附件绝不在本流程中删除。
///
/// Post-conditions:
/// - 受 Ledger 管辖的 `.md` 文件被无条件覆盖到规范内容。
/// - 不在 projection 中的陈旧 `.md` 会被删除。
pub(super) fn rebuild_local_repo(
    repo: &RepoManager,
    guard: &PersistGuard,
    repo_name: &str,
) -> Result<()> {
    let root = repo.local_repo_workspace_root(repo_name)?;
    std::fs::create_dir_all(&root)?;
    std::fs::create_dir_all(repo.local_repo_notegit_root(repo_name)?)?;
    let plan = projection_plan::build(repo, repo_name)?;
    rebuild_projection_state::rebuild_local_projection_state(repo, repo_name)?;
    for dir in &plan.dirs {
        if dir.is_empty() {
            continue;
        }
        std::fs::create_dir_all(repo.local_repo_workspace_path(repo_name, dir)?)?;
    }

    for (repo_path, doc_id) in &plan.docs {
        let file_path = repo.local_repo_workspace_path(repo_name, repo_path)?;
        let rebuilt = rebuild::rebuild_local_doc_in_repo(repo, repo_name, *doc_id)?;
        if let Some(parent) = file_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let root_relative = repo.local_repo_workspace_relative(repo_name, repo_path);
        guard.record(&root_relative, &rebuilt.content);
        if let Err(err) = std::fs::write(&file_path, rebuilt.content) {
            guard.clear(&root_relative);
            return Err(err.into());
        }
        repo.bind_workspace_inode_in_local_repo(repo_name, repo_path, *doc_id)?;
    }

    prune_stale_paths(repo_name, guard, &root, &plan.docs, &plan.dirs, "")?;
    Ok(())
}

fn prune_stale_paths(
    repo_name: &str,
    guard: &PersistGuard,
    root: &Path,
    expected_docs: &HashMap<String, crate::models::DocId>,
    expected_dirs: &HashSet<String>,
    relative: &str,
) -> Result<bool> {
    let current = if relative.is_empty() {
        root.to_path_buf()
    } else {
        root.join(relative)
    };
    for entry in std::fs::read_dir(&current)? {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().to_string();
        if name == ".notegit" {
            continue;
        }
        let child_rel = if relative.is_empty() {
            name
        } else {
            format!("{relative}/{name}")
        };
        let path = entry.path();
        if entry.file_type()?.is_dir() {
            let empty = prune_stale_paths(
                repo_name,
                guard,
                root,
                expected_docs,
                expected_dirs,
                &child_rel,
            )?;
            if empty && !expected_dirs.contains(&child_rel) {
                std::fs::remove_dir(&path)?;
            }
            continue;
        }
        let is_md = path.extension().and_then(|ext| ext.to_str()) == Some("md");
        if is_md && !expected_docs.contains_key(&child_rel) {
            let root_relative = format!("{repo_name}/{child_rel}");
            guard.record_delete(&root_relative);
            if let Err(err) = std::fs::remove_file(path) {
                guard.clear(&root_relative);
                return Err(err.into());
            }
        }
    }
    let root_rel = relative.is_empty() || expected_dirs.contains(relative);
    Ok(!root_rel && std::fs::read_dir(&current)?.next().is_none())
}
