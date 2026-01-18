// crates\core\src\sync
//! # 文件系统事件处理器 (FS Event Handler)
//!
//! 处理 Watcher 产生的文件变更事件。
//! 分离处理：删除、重命名、内容更新、新文件。

use crate::ledger::RepoManager;
use crate::ledger::listing::RepoListing;
use crate::models::RepoType;
use crate::protocol::ServerMessage;
use crate::sync::recovery;
use crate::vfs::Vfs;
use anyhow::Result;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tracing::{info, warn};

pub struct FsEventHandler<'a> {
    pub repo: &'a Arc<RepoManager>,
    pub vfs: &'a Vfs,
    pub vault_root: &'a Path,
}

impl<'a> FsEventHandler<'a> {
    pub fn new(repo: &'a Arc<RepoManager>, vfs: &'a Vfs, vault_root: &'a Path) -> Self {
        Self {
            repo,
            vfs,
            vault_root,
        }
    }

    pub fn handle_event(
        &self,
        path_str: &str,
        sync_mgr: &crate::sync::SyncManager,
    ) -> Result<Vec<ServerMessage>> {
        let file_path = self.vault_root.join(path_str);

        // CASE 1: File Deleted (or moved out of scope)
        if !file_path.exists() {
            if let Some(doc_id) = self.repo.get_docid(path_str)? {
                warn!(
                    "Handler: File gone: {}. Marking as deleted in Ledger.",
                    path_str
                );

                // 1. Remove from Path Mapping (metadata) to prevent "Ghost Files"
                // This ensures next scan won't see it as a valid entry
                self.repo.delete_doc(path_str)?;

                // 2. Broadcast Deletion to Peers
                // TODO: Protocol needs DocDeleted message.
                // Currently we just update local state. The sync engine should propagate
                // this as a "Tombstone" op if we are doing CRDT deletions.
                // But for now, ensuring metadata is clean prevents local issues.

                return Ok(vec![ServerMessage::DocDeleted { doc_id }]);
            }
            return Ok(vec![]);
        }

        let inode = match self.vfs.get_inode(path_str)? {
            Some(i) => i,
            None => return Ok(vec![]),
        };

        // CASE 2: Known Inode (Rename or Content Update)
        if let Some(doc_id) = self.repo.get_docid_by_inode(&inode)? {
            // 2a. Check Path (Rename)
            if let Some(known_path) = self.repo.get_path_by_docid(doc_id)? {
                if known_path != path_str {
                    info!("Handler: Rename detected {} -> {}", known_path, path_str);
                    self.repo.rename_doc(&known_path, path_str)?;
                    return self.gen_list();
                }
            }

            // 2b. Same Path => Content Update
            if sync_mgr.reconcile_doc(doc_id)? {
                return self.gen_list();
            }
            return Ok(vec![]);
        }

        // CASE 3: Unknown Inode (New File or Atomic Save)
        // 3a. Check Path in Ledger (Atomic Save / Rebind)
        if let Some(existing_id) = self.repo.get_docid(path_str)? {
            info!(
                "Handler: Inode change (Atomic Save?) for {}. Rebinding.",
                path_str
            );
            // RISK (Low): If user did "Delete A -> Create New A" quickly, we might bind New A content to Old A history.
            // Accepted trade-off for supporting Vim/Editor atomic saves (Write New + Rename).
            self.repo.bind_inode(&inode, existing_id)?;
            if sync_mgr.reconcile_doc(existing_id)? {
                // Content updated
            }
            return self.gen_list();
        }

        // 3b. Check Content for UUID (Recovery)
        let content = std::fs::read_to_string(&file_path)?;
        if let Some(recovered_id) = recovery::try_recover_from_content(&content) {
            if let Ok(Some(old_path)) = self.repo.get_path_by_docid(recovered_id) {
                info!(
                    "Handler: Recovery UUID found. Resurrecting {:?} from {} to {}",
                    recovered_id, old_path, path_str
                );

                if old_path != path_str {
                    self.repo.rename_doc(&old_path, path_str)?;
                }
                self.repo.bind_inode(&inode, recovered_id)?;
                let _ = sync_mgr.reconcile_doc(recovered_id);
                return self.gen_list();
            }
        }

        // 3c. Truly New File
        info!("Handler: New file detected: {}", path_str);
        let doc_id = self.repo.create_docid(path_str)?;
        self.repo.bind_inode(&inode, doc_id)?;

        // Initial ingest
        if !content.is_empty() {
            let now = chrono::Utc::now().timestamp_millis();
            let op = crate::models::Op::Insert { pos: 0, content };
            let entry = crate::models::LedgerEntry {
                doc_id,
                op,
                timestamp: now,
                peer_id: crate::models::PeerId::new("local_watcher"),
                seq: 0,
            };
            self.repo.append_local_op(&entry)?;
            info!("Handler: Ingested initial content.");
        }

        self.gen_list()
    }

    fn gen_list(&self) -> Result<Vec<ServerMessage>> {
        let docs = self.repo.list_docs(&RepoType::Local(uuid::Uuid::nil()))?;
        Ok(vec![ServerMessage::DocList { docs }])
    }
}
