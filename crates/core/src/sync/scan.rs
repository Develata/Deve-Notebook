// crates\core\src\sync
use crate::ledger::listing::RepoListing;
use crate::ledger::RepoManager;
use crate::models::RepoType;
use crate::utils::path::{path_to_forward_slash, to_forward_slash};
use crate::vfs::Vfs;
use anyhow::Result;
use std::path::Path;
use std::sync::Arc;
use tracing::{debug, error, info, warn};
use uuid::Uuid;
use walkdir::WalkDir;

/// Performs a full scan of the vault.
/// 1. Registers new files in Ledger.
/// 2. Binds Inodes.
/// 3. Removes ghost entries from Ledger.
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
                            && let Ok(rel_path) = entry.path().strip_prefix(vault_root) {
                                // 规范化路径：统一使用正斜杠格式（内部权威格式）
                                let path_str = path_to_forward_slash(rel_path);
                                on_disk_paths.insert(path_str.clone());

                                // Ensure DocID exists
                                let doc_id = if let Some(id) = repo.get_docid(&path_str)? {
                                    id
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
                "SyncScan: 检测到幽灵文件: {}（规范化后: {}），正在删除...",
                path, normalized_path
            );

            // SECURITY WARNING: Direct deletion based on scan result has a risk of metadata loss
            // if the FS is temporarily unavailable (e.g. unmounted drive).
            // Current assumption: Local disk availability is high.
            // Future improvement: Add a "Tombstone" state or second confirmation before physical deletion.

            // 尝试使用原始路径删除
            if let Err(e) = repo.delete_doc(&path) {
                warn!("使用原始路径删除失败 {}: {:?}", path, e);
            }
            // 如果路径不同，也尝试用规范化路径删除
            if normalized_path != path
                && let Err(e) = repo.delete_doc(&normalized_path) {
                    warn!("使用规范化路径删除失败 {}: {:?}", normalized_path, e);
                }
            info!("SyncScan: 幽灵文件删除完成: {}", path);
        }
    }

    info!("SyncScan: Scan complete.");
    Ok(())
}
