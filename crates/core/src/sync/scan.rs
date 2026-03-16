use crate::ledger::RepoManager;
use crate::models::DocId;
use crate::source_control::ChangeStatus;
use crate::utils::path::path_to_forward_slash;
use crate::vfs::Vfs;
use anyhow::{Result, anyhow};
use std::collections::HashSet;
use std::path::Path;
use std::sync::Arc;
use tracing::{info, warn};
use walkdir::WalkDir;

/// 全量扫描所有本地 repo 工作区：`vault/<repo_name>/...`
pub fn scan_vault(repo: &Arc<RepoManager>, vfs: &Vfs, vault_root: &Path) -> Result<()> {
    info!("SyncScan: Starting full scan of {:?}", vault_root);
    for repo_name in repo.list_local_repo_names_for_execution()? {
        scan_local_repo(repo, vfs, &repo_name)?;
    }
    info!("SyncScan: Scan complete.");
    Ok(())
}

pub(crate) fn scan_local_repo(repo: &Arc<RepoManager>, vfs: &Vfs, repo_name: &str) -> Result<()> {
    let repo_root = repo.local_repo_workspace_root(repo_name)?;
    std::fs::create_dir_all(&repo_root)?;
    let mut on_disk = HashSet::new();
    let mut seen_docs = HashSet::<DocId>::new();

    for entry in WalkDir::new(&repo_root) {
        let entry =
            entry.map_err(|err| anyhow!("Failed to walk local repo {}: {}", repo_name, err))?;
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

fn clear_scan_pending(repo: &Arc<RepoManager>, repo_name: &str, repo_path: &str) -> Result<()> {
    super::pending::clear(repo, repo_name, repo_path)
}
