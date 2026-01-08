use anyhow::Result;
use notify_debouncer_mini::{new_debouncer, notify::RecursiveMode};
use std::time::Duration;
use crate::ledger::Ledger;
use crate::vfs::Vfs;
use tracing::{info, warn, error};
use regex::Regex;

use std::sync::Arc;
use crate::models::DocId;

pub struct Watcher {
    ledger: Arc<Ledger>, // Shared
    vfs: Vfs,
}

impl Watcher {
    pub fn new(ledger: Arc<Ledger>, vfs: Vfs) -> Self {
        Self { ledger, vfs }
    }

    pub fn watch(&self) -> Result<()> {
        let (tx, rx) = std::sync::mpsc::channel();

        // 200ms debounce
        let mut debouncer = new_debouncer(Duration::from_millis(200), tx)?;

        debouncer
            .watcher()
            .watch(&self.vfs.root, RecursiveMode::Recursive)?;

        info!("Watcher started on {:?}", self.vfs.root);

        for result in rx {
            match result {
                Ok(events) => {
                    for event in events {
                       let path = event.path;
                       // Convert to relative string
                       if let Ok(rel) = path.strip_prefix(&self.vfs.root) {
                           let path_str = rel.to_string_lossy().to_string();
                           if let Err(e) = self.handle_event(&path_str) {
                               error!("Error handling event for {}: {:?}", path_str, e);
                           }
                       }
                    }
                }
                Err(e) => {
                    error!("Watch error: {:?}", e);
                }
            }
        }

        Ok(())
    }

    fn handle_event(&self, path_str: &str) -> Result<()> {
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
                        // Inode->DocId is constant, but we might want to ensure it.
                    } else {
                        // Same path, same inode. Content update?
                        // Diff Check & Create Op.
                        info!("Content update at {}", path_str);
                        
                        let new_content = std::fs::read_to_string(&self.vfs.root.join(path_str))?;
                        let ops_with_seq = self.ledger.get_ops(doc_id)?;
                        let ops: Vec<crate::models::LedgerEntry> = ops_with_seq.iter().map(|(_, e)| e.clone()).collect();
                        let old_content = crate::state::reconstruct_content(&ops);
                        
                        if new_content != old_content {
                            let diff_ops = crate::state::compute_diff(&old_content, &new_content);
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
                     
                     // Now process content diff
                     let new_content = std::fs::read_to_string(&self.vfs.root.join(path_str))?;
                     let ops_with_seq = self.ledger.get_ops(existing_id)?;
                     let ops: Vec<crate::models::LedgerEntry> = ops_with_seq.iter().map(|(_, e)| e.clone()).collect();
                     let old_content = crate::state::reconstruct_content(&ops);
                     
                     if new_content != old_content {
                         let diff_ops = crate::state::compute_diff(&old_content, &new_content);
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
                    // Check if file has `uuid: <uuid>` in frontmatter
                    let content = std::fs::read_to_string(&self.vfs.root.join(path_str)).unwrap_or_default();
                    let mut recovered_id = None;
                    
                    // Quick Regex for "uuid: <uuid>"
                    // Assuming YAML Frontmatter
                    let re = Regex::new(r"(?m)^uuid:\s*([a-fA-F0-9-]{36})").unwrap();
                    if let Some(caps) = re.captures(&content) {
                        if let Some(uuid_str) = caps.get(1) {
                             if let Ok(uuid_val) = uuid::Uuid::parse_str(uuid_str.as_str()) {
                                 let potential_id = DocId::from_u128(uuid_val.as_u128());
                                 
                                 // Check if this ID exists in Ledger
                                 if let Ok(Some(old_path)) = self.ledger.get_path_by_docid(potential_id) {
                                     info!("UUID Fallback Success! Found DocId {:?} (previously at {}) now at {}. Resurrecting...", potential_id, old_path, path_str);
                                     
                                     // 1. Rename logic (Old Path -> New Path)
                                     // Only if Old Path != New Path (which it likely is, or Path Fallback would have caught it?)
                                     // Wait, Path Fallback catches "Same Path, New Inode".
                                     // This catches "Different Path, New Inode" (Move + Git Pull) OR "Same Path, New Inode" (redundant but safe).
                                     if old_path != path_str {
                                          self.ledger.rename_doc(&old_path, path_str)?;
                                     }
                                     
                                     // 2. Bind Inode
                                     self.ledger.bind_inode(&inode, potential_id)?;
                                     recovered_id = Some(potential_id);
                                 }
                             }
                        }
                    }

                    if let Some(existing_id) = recovered_id {
                         // Compute Diff (same as above) - DRY this later
                         let ops_with_seq = self.ledger.get_ops(existing_id)?;
                         let ops: Vec<crate::models::LedgerEntry> = ops_with_seq.iter().map(|(_, e)| e.clone()).collect();
                         let old_content = crate::state::reconstruct_content(&ops);
                         
                         if content != old_content {
                             let diff_ops = crate::state::compute_diff(&old_content, &content);
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
            // Wait, if it was a rename, we might see Delete(Old) then Create(New).
            // But we process Create(New) above and handle the rename.
            // If it is a pure delete, Inode is gone.
            // We check if Path was known.
            if let Some(_doc_id) = self.ledger.get_docid(path_str)? {
                 // Known path is gone.
                 // Verify if it moved? 
                 // If we processed the "Rename/Move" event first, `get_docid(path_str)` might already be None (if we updated ledger).
                 // But if we see Delete first...
                 // We can't know if it moved unless we see the destination.
                 // So we conservatively mark it as deleted or just wait.
                 // For Phase 0, let's log it.
                 warn!("File gone: {}", path_str);
            }
        }

        Ok(())
    }
}
