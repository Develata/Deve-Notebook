//! plan_ref:
//!   - 03_storage/watcher#watcher-contract
//!   - 04_repository#tree-projection-contract

use crate::ledger::RepoManager;
use crate::models::DocId;
use crate::source_control::ChangeStatus;
use crate::utils::path::path_to_forward_slash;
use crate::vfs::Vfs;
use crate::watcher_ignore::IgnoreRules;
use anyhow::{Result, anyhow};
use std::collections::HashSet;
use std::sync::Arc;
use tracing::{info, warn};
use walkdir::WalkDir;

/// 全量扫描所有已绑定 Projection Locator 的本地 repo 工作区。
pub fn scan_projection_workspaces(repo: &Arc<RepoManager>, vfs: &Vfs) -> Result<()> {
    scan_all_local_repos_excluding(repo, vfs, &HashSet::new())
}

pub(crate) fn scan_all_local_repos_excluding(
    repo: &Arc<RepoManager>,
    vfs: &Vfs,
    excluded_repos: &HashSet<String>,
) -> Result<()> {
    info!("SyncScan: Starting full scan of local Projection Workspaces");
    for repo_name in repo.list_local_repo_names_for_execution()? {
        if excluded_repos.contains(&repo_name) {
            warn!(
                repo_name = %repo_name,
                "SyncScan: Skipping degraded local repo"
            );
            continue;
        }
        scan_local_repo(repo, vfs, &repo_name)?;
    }
    info!("SyncScan: Scan complete.");
    Ok(())
}

pub(crate) fn scan_local_repo(repo: &Arc<RepoManager>, vfs: &Vfs, repo_name: &str) -> Result<()> {
    let repo_root = repo.local_repo_workspace_root(repo_name)?;
    std::fs::create_dir_all(&repo_root)?;
    let ignore_rules = Some(IgnoreRules::load(&repo_root));
    let mut on_disk = HashSet::new();
    let mut seen_docs = HashSet::<DocId>::new();

    let walker = WalkDir::new(&repo_root).into_iter().filter_entry(|entry| {
        entry
            .file_name()
            .to_str()
            .is_none_or(|name| !crate::utils::notegit::is_internal_repo_segment(name))
    });
    for entry in walker {
        let entry =
            entry.map_err(|err| anyhow!("Failed to walk local repo {}: {}", repo_name, err))?;
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("md") {
            continue;
        }
        let repo_path = repo_relative_path(repo_name, &repo_root, path)?;
        if crate::utils::notegit::is_internal_repo_path(&repo_path) {
            continue;
        }
        if ignored_by_rules(repo, repo_name, &repo_path, ignore_rules.as_ref()) {
            continue;
        }
        if !entry.file_type().is_file() {
            return Err(anyhow!(
                "Broken local repo {} while scanning workspace: markdown path is not a file: {}",
                repo_name,
                path.display()
            ));
        }
        on_disk.insert(repo_path.clone());
        if let Some(doc_id) = super::scan_file::scan_disk_file(repo, vfs, repo_name, &repo_path)? {
            seen_docs.insert(doc_id);
        }
    }

    info!(
        "SyncScan: Repo {} 磁盘上发现 {} 个 md 文件",
        repo_name,
        on_disk.len()
    );

    let docs = repo.list_local_docs(Some(repo_name))?;
    info!(
        "SyncScan: Repo {} Ledger 中有 {} 个条目",
        repo_name,
        docs.len()
    );
    for (doc_id, repo_path) in docs {
        if crate::utils::notegit::is_internal_repo_path(&repo_path) {
            warn!("SyncScan: Repo {} 跳过内部路径: {}", repo_name, repo_path);
            continue;
        }
        if ignored_by_rules(repo, repo_name, &repo_path, ignore_rules.as_ref()) {
            continue;
        }
        if seen_docs.contains(&doc_id) || on_disk.contains(&repo_path) {
            continue;
        }
        info!("SyncScan: Repo {} 检测到幽灵文件: {}", repo_name, repo_path);
        super::pending::upsert(
            repo,
            repo_name,
            &repo_path,
            ChangeStatus::Deleted,
            Some(doc_id),
            None,
        )?;
    }

    for pending in repo.list_pending_fs_in_local_repo(repo_name)? {
        if pending.status == ChangeStatus::Added && !on_disk.contains(&pending.path) {
            clear_scan_pending(repo, repo_name, &pending.path)?;
        }
    }
    Ok(())
}

fn ignored_by_rules(
    repo: &RepoManager,
    repo_name: &str,
    repo_path: &str,
    rules: Option<&IgnoreRules>,
) -> bool {
    rules.is_some_and(|rules| {
        let root_relative = repo.local_repo_workspace_relative(repo_name, repo_path);
        rules.is_ignored_workspace_path(&root_relative, repo_path)
    })
}

fn clear_scan_pending(repo: &Arc<RepoManager>, repo_name: &str, repo_path: &str) -> Result<()> {
    super::pending::clear(repo, repo_name, repo_path)
}

fn repo_relative_path(
    repo_name: &str,
    repo_root: &std::path::Path,
    path: &std::path::Path,
) -> Result<String> {
    let rel = path.strip_prefix(repo_root).map_err(|_| {
        anyhow!(
            "Broken local repo {} while scanning workspace: path escaped repo root: {}",
            repo_name,
            path.display()
        )
    })?;
    Ok(path_to_forward_slash(rel))
}

#[cfg(test)]
mod tests {
    use super::repo_relative_path;
    use std::path::Path;

    #[test]
    fn repo_relative_path_fails_closed_when_path_escapes_repo_root() {
        let err = repo_relative_path(
            "default",
            Path::new("/projection/default"),
            Path::new("/tmp/a.md"),
        )
        .expect_err("escaped path must fail closed");
        assert!(err.to_string().contains("path escaped repo root"));
    }
}
