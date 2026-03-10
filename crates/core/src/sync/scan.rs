use super::rebuild;
use crate::ledger::RepoManager;
use crate::ledger::listing::RepoListing;
use crate::source_control::ChangeStatus;
use crate::utils::path::path_to_forward_slash;
use crate::vfs::Vfs;
use anyhow::Result;
use std::collections::HashSet;
use std::path::Path;
use std::sync::Arc;
use tracing::{info, warn};
use walkdir::WalkDir;

/// 全量扫描所有本地 repo 工作区：`vault/<repo_name>/...`
pub fn scan_vault(repo: &Arc<RepoManager>, vfs: &Vfs, vault_root: &Path) -> Result<()> {
    info!("SyncScan: Starting full scan of {:?}", vault_root);
    for repo_name in repo.list_repos(None)? {
        scan_local_repo(repo, vfs, &repo_name)?;
    }
    info!("SyncScan: Scan complete.");
    Ok(())
}

fn scan_local_repo(repo: &Arc<RepoManager>, vfs: &Vfs, repo_name: &str) -> Result<()> {
    let repo_root = repo.local_repo_workspace_root(repo_name)?;
    std::fs::create_dir_all(&repo_root)?;
    let mut on_disk = HashSet::new();

    for entry in WalkDir::new(&repo_root).into_iter().filter_map(Result::ok) {
        let path = entry.path();
        if !path.is_file() || path.extension().and_then(|ext| ext.to_str()) != Some("md") {
            continue;
        }
        let Ok(rel) = path.strip_prefix(&repo_root) else {
            continue;
        };
        let repo_path = path_to_forward_slash(rel);
        if crate::utils::notegit::is_internal_repo_path(&repo_path) {
            continue;
        }
        on_disk.insert(repo_path.clone());
        scan_disk_file(repo, vfs, repo_name, &repo_path)?;
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
        if on_disk.contains(&repo_path) {
            continue;
        }
        info!("SyncScan: Repo {} 检测到幽灵文件: {}", repo_name, repo_path);
        super::pending::upsert(
            repo,
            repo_name,
            &repo_path,
            ChangeStatus::Deleted,
            Some(doc_id),
        )?;
    }

    for pending in repo.list_pending_fs_in_local_repo(repo_name)? {
        if pending.status == ChangeStatus::Added && !on_disk.contains(&pending.path) {
            clear_scan_pending(repo, repo_name, &pending.path)?;
        }
    }
    Ok(())
}

fn scan_disk_file(
    repo: &Arc<RepoManager>,
    vfs: &Vfs,
    repo_name: &str,
    repo_path: &str,
) -> Result<()> {
    let existing = repo.get_docid_in_local_repo(repo_name, repo_path)?;
    let Some(doc_id) = existing else {
        return super::pending::upsert(repo, repo_name, repo_path, ChangeStatus::Added, None);
    };

    let root_rel = repo.local_repo_workspace_relative(repo_name, repo_path);
    if let Ok(Some(inode)) = vfs.get_inode(&root_rel) {
        repo.bind_inode_in_local_repo(repo_name, &inode, doc_id)?;
    }

    let disk_path = repo.local_repo_workspace_path(repo_name, repo_path)?;
    let disk_content = std::fs::read_to_string(&disk_path).unwrap_or_default();
    let rebuilt = rebuild::rebuild_local_doc_in_repo(repo, repo_name, doc_id)?;
    if rebuilt.content == disk_content {
        clear_scan_pending(repo, repo_name, repo_path)
    } else {
        super::pending::upsert(
            repo,
            repo_name,
            repo_path,
            ChangeStatus::Modified,
            Some(doc_id),
        )
    }
}

fn clear_scan_pending(repo: &Arc<RepoManager>, repo_name: &str, repo_path: &str) -> Result<()> {
    super::pending::clear(repo, repo_name, repo_path)
}
