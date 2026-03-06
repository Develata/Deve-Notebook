// crates\core\src\sync
use crate::ledger::RepoManager;
use crate::ledger::listing::RepoListing;
use crate::models::RepoType;
use crate::source_control::ChangeStatus;
use crate::source_control::pending_fs::{self, PendingFsEntry};
use crate::utils::path::{path_to_forward_slash, to_forward_slash};
use crate::vfs::Vfs;
use anyhow::Result;
use std::path::Path;
use std::sync::Arc;
use tracing::{debug, error, info, warn};
use uuid::Uuid;
use walkdir::WalkDir;

/// 全量扫描 Vault 目录。
/// 1. 注册新文件的 DocID 并绑定 Inode。
/// 2. 将磁盘上发现的文件记录为 pending_fs_ops（供 Stage → Commit 流程使用）。
/// 3. 清理 Ledger 中的幽灵条目（磁盘上已不存在的文件），并记录 Deleted pending。
pub fn scan_vault(repo: &Arc<RepoManager>, vfs: &Vfs, vault_root: &Path) -> Result<()> {
    info!("SyncScan: Starting full scan of {:?}", vault_root);

    // 1. Scan Disk -> Ledger
    let walker = WalkDir::new(vault_root).into_iter();
    let mut on_disk_paths = std::collections::HashSet::new();

    for entry in walker.filter_entry(|e| !e.file_name().to_string_lossy().starts_with('.')) {
        match entry {
            Ok(entry) => {
                if entry.file_type().is_file()
                    && let Some(ext) = entry.path().extension()
                    && ext == "md"
                    && let Ok(rel_path) = entry.path().strip_prefix(vault_root)
                {
                    // 规范化路径：统一使用正斜杠格式（内部权威格式）
                    let path_str = path_to_forward_slash(rel_path);
                    on_disk_paths.insert(path_str.clone());

                    // Ensure DocID exists
                    let is_new = repo.get_docid(&path_str)?.is_none();
                    let doc_id = if !is_new {
                        repo.get_docid(&path_str)?.unwrap()
                    } else {
                        if let Err(e) = repo.create_docid(&path_str) {
                            error!("Failed to register {}: {:?}", path_str, e);
                            continue;
                        }
                        repo.get_docid(&path_str)?.unwrap()
                    };

                    // Bind Inode
                    if let Ok(Some(inode)) = vfs.get_inode(&path_str) {
                        let _ = repo.bind_inode(&inode, doc_id);
                    }

                    // 记录到 pending_fs_ops（新文件 Added，已有文件 Modified）
                    let status = if is_new {
                        ChangeStatus::Added
                    } else {
                        ChangeStatus::Modified
                    };
                    let hash = upsert_scan_pending(repo, vault_root, &path_str, status);
                    if let Err(e) = hash {
                        warn!("SyncScan: pending upsert 失败 {}: {:?}", path_str, e);
                    }
                }
            }
            Err(e) => warn!("Walk error: {:?}", e),
        }
    }
    info!("SyncScan: 磁盘上发现 {} 个 md 文件", on_disk_paths.len());
    debug!("SyncScan: On-disk paths: {:?}", on_disk_paths);

    // 2. Scan Ledger -> Disk (Cleanup Ghosts)
    let repo_id = repo
        .get_repo_info()?
        .map(|info| info.uuid)
        .unwrap_or_else(Uuid::nil);
    let docs = repo.list_docs(&RepoType::Local(repo_id))?;
    info!("SyncScan: Ledger 中有 {} 个条目", docs.len());

    for (doc_id, path) in docs {
        debug!("SyncScan: Ledger 条目: {} (DocId: {})", path, doc_id);
        let normalized_path = to_forward_slash(&path);
        if !on_disk_paths.contains(&normalized_path) {
            info!(
                "SyncScan: 检测到幽灵文件: {}（规范化后: {}），记录 Deleted pending...",
                path, normalized_path
            );

            // 记录 Deleted 到 pending_fs_ops（用户可在 UI 中确认）
            let entry = PendingFsEntry {
                path: normalized_path.clone(),
                change_type: ChangeStatus::Deleted,
                content_hash: String::new(),
                detected_at: chrono::Utc::now().timestamp_millis(),
                has_conflict: false,
            };
            if let Err(e) = pending_fs::upsert(&repo.local_db, &entry) {
                warn!("SyncScan: pending upsert (deleted) 失败 {}: {:?}", path, e);
            }

            // 清理 Ledger 中的元数据
            if let Err(e) = repo.delete_doc(&path) {
                warn!("使用原始路径删除失败 {}: {:?}", path, e);
            }
            if normalized_path != path
                && let Err(e) = repo.delete_doc(&normalized_path)
            {
                warn!("使用规范化路径删除失败 {}: {:?}", normalized_path, e);
            }
            info!("SyncScan: 幽灵文件处理完成: {}", path);
        }
    }

    info!("SyncScan: Scan complete.");
    Ok(())
}

/// 辅助函数：将扫描发现的文件记录到 pending_fs_ops
fn upsert_scan_pending(
    repo: &Arc<RepoManager>,
    vault_root: &Path,
    path_str: &str,
    status: ChangeStatus,
) -> Result<()> {
    let hash = if status == ChangeStatus::Deleted {
        String::new()
    } else {
        let file_path = vault_root.join(path_str);
        let content = std::fs::read_to_string(&file_path).unwrap_or_default();
        pending_fs::content_hash(&content)
    };
    let entry = PendingFsEntry {
        path: path_str.to_string(),
        change_type: status,
        content_hash: hash,
        detected_at: chrono::Utc::now().timestamp_millis(),
        has_conflict: false,
    };
    pending_fs::upsert(&repo.local_db, &entry)
}
