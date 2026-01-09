use std::path::PathBuf;
use std::sync::Arc;
use anyhow::Result;
use tracing::{info, warn, error};
use walkdir::WalkDir;
use crate::ledger::Ledger;
use crate::models::{DocId, LedgerEntry};
use crate::vfs::Vfs;
use regex::Regex;

pub struct SyncManager {
    ledger: Arc<Ledger>,
    vault_root: PathBuf,
    vfs: Vfs,
}

impl SyncManager {
    pub fn new(ledger: Arc<Ledger>, vault_root: PathBuf) -> Self {
        let vfs = Vfs::new(&vault_root);
        Self { ledger, vault_root, vfs }
    }

    /// Full Scan: Syncs Ledger with Disk on Startup
    /// 1. Adds missing files to Ledger.
    /// 2. Removes ghost files (in Ledger but not on Disk).
    pub fn scan(&self) -> Result<()> {
        info!("SyncManager: Starting full scan of {:?}", self.vault_root);
        
        // 1. Scan Disk -> Ledger
        let walker = WalkDir::new(&self.vault_root).into_iter();
        let mut on_disk_paths = std::collections::HashSet::new();

        for entry in walker.filter_entry(|e| !e.file_name().to_string_lossy().starts_with('.')) {
            match entry {
                Ok(entry) => {
                    if entry.file_type().is_file() {
                        if let Some(ext) = entry.path().extension() {
                            if ext == "md" {
                                if let Ok(rel_path) = entry.path().strip_prefix(&self.vault_root) {
                                    let path_str = rel_path.to_string_lossy().to_string();
                                    on_disk_paths.insert(path_str.clone());

                                    // Ensure DocID exists
                                    let doc_id = if let Some(id) = self.ledger.get_docid(&path_str)? {
                                        id
                                    } else {
                                        if let Err(e) = self.ledger.create_docid(&path_str) {
                                            error!("Failed to register {}: {:?}", path_str, e);
                                            continue;
                                        }
                                        self.ledger.get_docid(&path_str)?.unwrap()
                                    };

                                    // Bind Inode during scan too!
                                    if let Ok(Some(inode)) = self.vfs.get_inode(&path_str) {
                                         let _ = self.ledger.bind_inode(&inode, doc_id);
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
        let docs = self.ledger.list_docs()?;
        for (_doc_id, path) in docs {
            if !on_disk_paths.contains(&path) {
                // info!("SyncManager: Ghost file detected (in Ledger, not on Disk): {}. Removing...", path);
                // Silent cleanup for now, or log
                if let Err(e) = self.ledger.delete_doc(&path) {
                    error!("Failed to remove ghost doc {}: {:?}", path, e);
                }
            }
        }

        info!("SyncManager: Scan complete.");
        Ok(())
    }

    /// Reconcile: Ensures Ledger content matches Disk content for a specific Doc
    /// Returns true if changes were applied.
    pub fn reconcile_doc(&self, doc_id: DocId) -> Result<bool> {
        if let Some(path_str) = self.ledger.get_path_by_docid(doc_id)? {
            let file_path = self.vault_root.join(&path_str);

            if file_path.exists() && file_path.is_file() {
                let disk_content = std::fs::read_to_string(&file_path)?;
                // Normalize for comparison
                let disk_norm = disk_content.replace("\r\n", "\n");

                // Get Current Ledger Content
                let ops = self.ledger.get_ops(doc_id)?;
                let ledger_ops: Vec<LedgerEntry> = ops.into_iter().map(|(_, e)| e).collect();
                let ledger_content = crate::state::reconstruct_content(&ledger_ops);
                let ledger_norm = ledger_content.replace("\r\n", "\n");

                if disk_norm != ledger_norm {
                    info!("SyncManager: Content mismatch for {}. reconciling...", path_str);
                    let new_ops = crate::state::compute_diff(&ledger_norm, &disk_norm);
                    
                    if !new_ops.is_empty() {
                         info!("SyncManager: Applying {} fix ops.", new_ops.len());
                         let now = chrono::Utc::now().timestamp_millis();
                         for op in new_ops {
                             let entry = LedgerEntry {
                                 doc_id,
                                 op,
                                 timestamp: now,
                             };
                             self.ledger.append_op(&entry)?;
                         }
                         return Ok(true);
                    }
                }
            } else {
                warn!("SyncManager: File not found during reconcile: {:?}", file_path);
            }
        }
        Ok(false)
    }

    /// Persist: Writes Ledger content to Disk (User Edit)
    pub fn persist_doc(&self, doc_id: DocId) -> Result<()> {
        if let Some(path_str) = self.ledger.get_path_by_docid(doc_id)? {
            let file_path = self.vault_root.join(&path_str);
            
            // Reconstruct
            let ops = self.ledger.get_ops(doc_id)?;
            let ledger_ops: Vec<LedgerEntry> = ops.into_iter().map(|(_, e)| e).collect();
            let content = crate::state::reconstruct_content(&ledger_ops);

            // Write
            std::fs::write(&file_path, content)?;
            info!("SyncManager: Persisted doc {} to {:?}", doc_id, file_path);
        }
        Ok(())
    }

    pub fn handle_fs_event(&self, path_str: &str) -> Result<()> {
        info!("SyncManager: Handling FS event for {}", path_str);

        if let Some(inode) = self.vfs.get_inode(path_str)? {
            // File EXISTS. 
            // Check if we know this Inode.
            if let Some(doc_id) = self.ledger.get_docid_by_inode(&inode)? {
                // Known Inode.
                // Check if Path matches.
                if let Some(known_path) = self.ledger.get_path_by_docid(doc_id)? {
                    if known_path != path_str {
                        // Inode matches, but Path differs. -> RENAME DETECTED.
                        info!("Rename detected: {} -> {}", known_path, path_str);
                        self.ledger.rename_doc(&known_path, path_str)?;
                        // Update Inode binding just in case? Inode is key, DocId is value.
                    } else {
                        // Same path, same inode. Content update?
                        // Diff Check & Create Op.
                        // info!("Content update check at {}", path_str);
                        
                        let new_content = std::fs::read_to_string(&self.vault_root.join(path_str))?;
                        let ops_with_seq = self.ledger.get_ops(doc_id)?;
                        let ops: Vec<crate::models::LedgerEntry> = ops_with_seq.iter().map(|(_, e)| e.clone()).collect();
                        let old_content = crate::state::reconstruct_content(&ops);
                        
                        // Normalize
                        let new_norm = new_content.replace("\r\n", "\n");
                        let old_norm = old_content.replace("\r\n", "\n");

                        if new_norm != old_norm {
                            let diff_ops = crate::state::compute_diff(&old_norm, &new_norm);
                            if !diff_ops.is_empty() {
                                info!("Detected {} changes. Appending to Ledger.", diff_ops.len());
                                let now = chrono::Utc::now().timestamp_millis();
                                for op in diff_ops {
                                    let entry = crate::models::LedgerEntry {
                                        doc_id,
                                        op,
                                        timestamp: now,
                                    };
                                    self.ledger.append_op(&entry)?;
                                }
                            }
                        }
                    }
                }
            } else {
                // UTTERLY NEW INODE.
                // Fallback 1: Path-based Recovery (Atomic Save / Inode Recycle)
                if let Some(existing_id) = self.ledger.get_docid(path_str)? {
                     // Path is known! This means the Inode changed (e.g. vim atomic save).
                     // We should REBIND the new Inode to the old DocId.
                     info!("Inode change detected (Atomic Save?) for {}. Rebinding {:?} -> {:?}", path_str, inode, existing_id);
                     self.ledger.bind_inode(&inode, existing_id)?;
                     
                     // Now process content diff (DRY needed, but copy for now)
                     let new_content = std::fs::read_to_string(&self.vault_root.join(path_str))?;
                     let ops_with_seq = self.ledger.get_ops(existing_id)?;
                     let ops: Vec<crate::models::LedgerEntry> = ops_with_seq.iter().map(|(_, e)| e.clone()).collect();
                     let old_content = crate::state::reconstruct_content(&ops);
                     
                     let new_norm = new_content.replace("\r\n", "\n");
                     let old_norm = old_content.replace("\r\n", "\n");

                     if new_norm != old_norm {
                         let diff_ops = crate::state::compute_diff(&old_norm, &new_norm);
                         if !diff_ops.is_empty() {
                             info!("Detected {} changes during rebind. Appending.", diff_ops.len());
                             let now = chrono::Utc::now().timestamp_millis();
                             for op in diff_ops {
                                 let entry = crate::models::LedgerEntry {
                                     doc_id: existing_id,
                                     op,
                                     timestamp: now,
                                 };
                                 self.ledger.append_op(&entry)?;
                             }
                         }
                     }
                } else {
                    // Fallback 2: UUID/Frontmatter Recovery
                    let content = std::fs::read_to_string(&self.vault_root.join(path_str)).unwrap_or_default();
                    let mut recovered_id = None;
                    
                    let re = Regex::new(r"(?m)^uuid:\s*([a-fA-F0-9-]{36})").unwrap();
                    if let Some(caps) = re.captures(&content) {
                        if let Some(uuid_str) = caps.get(1) {
                             if let Ok(uuid_val) = uuid::Uuid::parse_str(uuid_str.as_str()) {
                                 let potential_id = DocId::from_u128(uuid_val.as_u128());
                                 
                                 // Check if this ID exists in Ledger
                                 if let Ok(Some(old_path)) = self.ledger.get_path_by_docid(potential_id) {
                                     info!("UUID Fallback Success! Found DocId {:?} (previously at {}) now at {}. Resurrecting...", potential_id, old_path, path_str);
                                     
                                     if old_path != path_str {
                                          self.ledger.rename_doc(&old_path, path_str)?;
                                     }
                                     
                                     self.ledger.bind_inode(&inode, potential_id)?;
                                     recovered_id = Some(potential_id);
                                 }
                             }
                        }
                    }

                    if let Some(existing_id) = recovered_id {
                         // Content sync after recovery
                         let ops_with_seq = self.ledger.get_ops(existing_id)?;
                         let ops: Vec<crate::models::LedgerEntry> = ops_with_seq.iter().map(|(_, e)| e.clone()).collect();
                         let old_content = crate::state::reconstruct_content(&ops);
                         
                         let new_norm = content.replace("\r\n", "\n");
                         let old_norm = old_content.replace("\r\n", "\n");
                         
                         if new_norm != old_norm {
                             let diff_ops = crate::state::compute_diff(&old_norm, &new_norm);
                             if !diff_ops.is_empty() {
                                 info!("Detected {} changes during UUID recovery. Appending.", diff_ops.len());
                                 let now = chrono::Utc::now().timestamp_millis();
                                 for op in diff_ops {
                                     let entry = crate::models::LedgerEntry {
                                         doc_id: existing_id,
                                         op,
                                         timestamp: now,
                                     };
                                     self.ledger.append_op(&entry)?;
                                 }
                             }
                         }
                    } else {
                        // True New File
                        info!("New file detected: {}", path_str);
                        let doc_id = self.ledger.create_docid(path_str)?;
                        self.ledger.bind_inode(&inode, doc_id)?;
                        
                        // Ingest initial content
                        if !content.is_empty() {
                            let now = chrono::Utc::now().timestamp_millis();
                            let op = crate::models::Op::Insert { pos: 0, content };
                            let entry = crate::models::LedgerEntry {
                                doc_id,
                                op,
                                timestamp: now,
                            };
                            self.ledger.append_op(&entry)?;
                            info!("Ingested initial content for {}", path_str);
                        }
                    }
                }
            }
        } else {
            // File does NOT exist. DELETION?
            // If path known, mark/log?
            if let Some(_doc_id) = self.ledger.get_docid(path_str)? {
                 // Known path is gone.
                 warn!("File gone: {}", path_str);
                 // Optional: self.ledger.delete_doc(path_str)? 
                 // But we usually wait for explicit delete or next scan cleanup.
            }
        }

        Ok(())
    }
}
