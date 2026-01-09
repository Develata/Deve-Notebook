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
                                let path_str = rel_path.to_string_lossy().to_string();
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

    // 2. Scan Ledger -> Disk (Cleanup Ghosts)
    let docs = ledger.list_docs()?;
    for (_doc_id, path) in docs {
        if !on_disk_paths.contains(&path) {
            // info!("SyncScan: Ghost file detected: {}. Removing...", path);
            if let Err(e) = ledger.delete_doc(&path) {
                error!("Failed to remove ghost doc {}: {:?}", path, e);
            }
        }
    }

    info!("SyncScan: Scan complete.");
    Ok(())
}
