use super::rebuild;
use crate::ledger::RepoManager;
use crate::ledger::listing::RepoListing;
use crate::source_control::ChangeStatus;
use crate::source_control::pending_fs::{self, PendingFsEntry};
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
    for (_doc_id, repo_path) in docs {
        if crate::utils::notegit::is_internal_repo_path(&repo_path) {
            warn!(
                "SyncScan: Repo {} 清理误入账本的内部路径: {}",
                repo_name, repo_path
            );
            repo.delete_doc_in_local_repo(repo_name, &repo_path)?;
            continue;
        }
        if on_disk.contains(&repo_path) {
            continue;
        }
        info!("SyncScan: Repo {} 检测到幽灵文件: {}", repo_name, repo_path);
        upsert_scan_pending(repo, repo_name, &repo_path, ChangeStatus::Deleted)?;
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
        return upsert_scan_pending(repo, repo_name, repo_path, ChangeStatus::Added);
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
        upsert_scan_pending(repo, repo_name, repo_path, ChangeStatus::Modified)
    }
}

fn clear_scan_pending(repo: &Arc<RepoManager>, repo_name: &str, repo_path: &str) -> Result<()> {
    repo.run_on_local_repo(repo_name, |db| {
        let _ = pending_fs::remove(db, repo_path);
        Ok(())
    })
}

fn upsert_scan_pending(
    repo: &Arc<RepoManager>,
    repo_name: &str,
    repo_path: &str,
    status: ChangeStatus,
) -> Result<()> {
    let hash = if status == ChangeStatus::Deleted {
        String::new()
    } else {
        let file_path = repo.local_repo_workspace_path(repo_name, repo_path)?;
        let content = std::fs::read_to_string(&file_path).unwrap_or_default();
        pending_fs::content_hash(&content)
    };
    let entry = PendingFsEntry {
        path: repo_path.to_string(),
        change_type: status,
        content_hash: hash,
        detected_at: chrono::Utc::now().timestamp_millis(),
        has_conflict: false,
    };
    repo.run_on_local_repo(repo_name, |db| pending_fs::upsert(db, &entry))
        .map_err(|err| {
            warn!(
                "SyncScan: pending upsert 失败 {}:{} {:?}",
                repo_name, repo_path, err
            );
            err
        })
}
