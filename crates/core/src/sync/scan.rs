use std::path::Path;
use std::sync::Arc;
use walkdir::WalkDir;
use tracing::{info, warn, error};
use anyhow::Result;
use crate::ledger::Ledger;
use crate::vfs::Vfs;

/// Performs a full scan of the vault.
/// 1. Registers new files in Ledger.
/// 2. Binds Inodes.
/// 3. Removes ghost entries from Ledger.
pub fn scan_vault(
    ledger: &Arc<Ledger>, 
    vfs: &Vfs, 
    vault_root: &Path
) -> Result<()> {
    info!("SyncScan: Starting full scan of {:?}", vault_root);
    
    // 1. Scan Disk -> Ledger
    let walker = WalkDir::new(vault_root).into_iter();
    let mut on_disk_paths = std::collections::HashSet::new();

    for entry in walker.filter_entry(|e| !e.file_name().to_string_lossy().starts_with('.')) {
        match entry {
            Ok(entry) => {
                if entry.file_type().is_file() {
                    if let Some(ext) = entry.path().extension() {
                        if ext == "md" {
                            if let Ok(rel_path) = entry.path().strip_prefix(vault_root) {
                                // 规范化路径：Windows 反斜杠转换为正斜杠
                                let path_str = rel_path.to_string_lossy().replace("\\", "/");
                                on_disk_paths.insert(path_str.clone());

                                // Ensure DocID exists
                                let doc_id = if let Some(id) = ledger.get_docid(&path_str)? {
                                    id
                                } else {
                                    if let Err(e) = ledger.create_docid(&path_str) {
                                        error!("Failed to register {}: {:?}", path_str, e);
                                        continue;
                                    }
                                    ledger.get_docid(&path_str)?.unwrap()
                                };

                                // Bind Inode
                                if let Ok(Some(inode)) = vfs.get_inode(&path_str) {
                                     let _ = ledger.bind_inode(&inode, doc_id);
                                }
                            }
                        }
                    }
                }
            },
            Err(e) => warn!("Walk error: {:?}", e),
        }
    }
    // 调试：显示磁盘上的文件列表
    info!("SyncScan: 磁盘上发现 {} 个 md 文件", on_disk_paths.len());
    for path in &on_disk_paths {
        info!("  - 磁盘文件: {}", path);
    }

    // 2. Scan Ledger -> Disk (Cleanup Ghosts)
    let docs = ledger.list_docs()?;
    info!("SyncScan: Ledger 中有 {} 个条目", docs.len());
    
    for (doc_id, path) in docs {
        info!("  - Ledger 条目: {} (DocId: {})", path, doc_id);
        // 规范化 ledger 中的路径以确保一致比较
        let normalized_path = path.replace("\\", "/");
        if !on_disk_paths.contains(&normalized_path) {
            info!("SyncScan: 检测到幽灵文件: {}（规范化后: {}），正在删除...", path, normalized_path);
            // 尝试使用原始路径删除
            if let Err(e) = ledger.delete_doc(&path) {
                warn!("使用原始路径删除失败 {}: {:?}", path, e);
            }
            // 如果路径不同，也尝试用规范化路径删除
            if normalized_path != path {
                if let Err(e) = ledger.delete_doc(&normalized_path) {
                    warn!("使用规范化路径删除失败 {}: {:?}", normalized_path, e);
                }
            }
            info!("SyncScan: 幽灵文件删除完成: {}", path);
        }
    }

    info!("SyncScan: Scan complete.");
    Ok(())
}
