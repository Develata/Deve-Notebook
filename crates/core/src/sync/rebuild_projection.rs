use super::persist_guard::PersistGuard;
use super::rebuild;
use crate::ledger::RepoManager;
use crate::models::NodeKind;
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
    let expected_dirs = expected_dirs(repo, repo_name)?;
    for dir in &expected_dirs {
        if dir.is_empty() {
            continue;
        }
        std::fs::create_dir_all(repo.local_repo_workspace_path(repo_name, dir)?)?;
    }

    let expected_docs = expected_docs(repo, repo_name)?;
    for (repo_path, doc_id) in &expected_docs {
        let file_path = repo.local_repo_workspace_path(repo_name, repo_path)?;
        let rebuilt = rebuild::rebuild_local_doc_in_repo(repo, repo_name, *doc_id)?;
        if let Some(parent) = file_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        guard.record(
            &repo.local_repo_workspace_relative(repo_name, repo_path),
            &rebuilt.content,
        );
        std::fs::write(&file_path, rebuilt.content)?;
        repo.bind_workspace_inode_in_local_repo(repo_name, repo_path, *doc_id)?;
    }

    prune_stale_paths(&root, &expected_docs, &expected_dirs, "")?;
    Ok(())
}

fn expected_docs(
    repo: &RepoManager,
    repo_name: &str,
) -> Result<HashMap<String, crate::models::DocId>> {
    Ok(repo
        .list_local_docs(Some(repo_name))?
        .into_iter()
        .filter(|(_, path)| !crate::utils::notegit::is_internal_repo_path(path))
        .map(|(doc_id, path)| (path, doc_id))
        .collect())
}

fn expected_dirs(repo: &RepoManager, repo_name: &str) -> Result<HashSet<String>> {
    let mut dirs = HashSet::from([String::new()]);
    for (_node_id, meta) in repo.list_local_nodes(Some(repo_name))? {
        let path = meta.path.trim_matches('/').to_string();
        if crate::utils::notegit::is_internal_repo_path(&path) {
            continue;
        }
        if meta.kind == NodeKind::Dir && !path.is_empty() {
            dirs.insert(path.clone());
        }
        insert_parents(&mut dirs, &path);
    }
    Ok(dirs)
}

fn insert_parents(dirs: &mut HashSet<String>, path: &str) {
    let mut cursor = Path::new(path).parent();
    while let Some(parent) = cursor {
        let value = crate::utils::path::to_forward_slash(&parent.to_string_lossy());
        if value.is_empty() || !dirs.insert(value.clone()) {
            break;
        }
        cursor = parent.parent();
    }
}

fn prune_stale_paths(
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
        if path.is_dir() {
            let empty = prune_stale_paths(root, expected_docs, expected_dirs, &child_rel)?;
            if empty && !expected_dirs.contains(&child_rel) {
                std::fs::remove_dir(&path)?;
            }
            continue;
        }
        let is_md = path.extension().and_then(|ext| ext.to_str()) == Some("md");
        if is_md && !expected_docs.contains_key(&child_rel) {
            std::fs::remove_file(path)?;
        }
    }
    let root_rel = relative.is_empty() || expected_dirs.contains(relative);
    Ok(!root_rel && std::fs::read_dir(&current)?.next().is_none())
}
