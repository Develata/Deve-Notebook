use anyhow::Result;
use notify_debouncer_mini::{new_debouncer, notify::RecursiveMode};
use std::time::Duration;
use crate::ledger::Ledger;
use crate::vfs::Vfs;
use tracing::{info, warn, error};

pub struct Watcher {
    ledger: Ledger, // Shared? Or clone? Redb is thread-safe (Arc internal). Simple struct wrap is fine if cheap.
    vfs: Vfs,
}

impl Watcher {
    pub fn new(ledger: Ledger, vfs: Vfs) -> Self {
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
                        let ops = self.ledger.get_ops(doc_id)?;
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
                // UTTERLY NEW INODE -> New File (or copied from outside).
                info!("New file detected: {}", path_str);
                let doc_id = self.ledger.create_docid(path_str)?;
                self.ledger.bind_inode(&inode, doc_id)?;
                
                // Ingest initial content
                let content = std::fs::read_to_string(&self.vfs.root.join(path_str))?;
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
