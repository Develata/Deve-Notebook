// crates\core\src\sync
//! # 文件系统事件处理器 (FS Event Handler)
//!
//! 处理 Watcher 产生的文件变更事件。
//! 分离处理：删除、重命名、内容更新、新文件。

use crate::ledger::RepoManager;
use crate::ledger::listing::RepoListing;
use crate::models::RepoType;
use crate::protocol::ServerMessage;
use crate::source_control::pending_fs::{self, PendingFsEntry};
use crate::source_control::ChangeStatus;
use crate::sync::recovery;
use crate::vfs::Vfs;
use anyhow::Result;
use std::path::Path;
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
        _sync_mgr: &crate::sync::SyncManager,
    ) -> Result<Vec<ServerMessage>> {
        let file_path = self.vault_root.join(path_str);

        // CASE 1: File Deleted (or moved out of scope)
        if !file_path.exists() {
            if let Some(_doc_id) = self.repo.get_docid(path_str)? {
                warn!("Handler: File gone: {}. Recording as pending delete.", path_str);
                self.repo.delete_doc(path_str)?;
                self.upsert_pending(path_str, ChangeStatus::Deleted)?;
                let mut msgs = self.gen_list()?;
                msgs.push(self.fs_change_msg(path_str, "deleted"));
                return Ok(msgs);
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
            if let Some(known_path) = self.repo.get_path_by_docid(doc_id)?
                && known_path != path_str
            {
                info!("Handler: Rename detected {} -> {}", known_path, path_str);
                self.repo.rename_doc(&known_path, path_str)?;
                return self.gen_list();
            }

            // 2b. Same Path => Content Update (记录为 pending，不自动 ingest)
            info!("Handler: Content update detected for {}", path_str);
            self.upsert_pending(path_str, ChangeStatus::Modified)?;
            let mut msgs = self.gen_list()?;
            msgs.push(self.fs_change_msg(path_str, "modified"));
            return Ok(msgs);
        }

        // CASE 3: Unknown Inode (New File or Atomic Save)
        // 3a. Check Path in Ledger (Atomic Save / Rebind)
        if let Some(existing_id) = self.repo.get_docid(path_str)? {
            info!(
                "Handler: Inode change (Atomic Save?) for {}. Rebinding.",
                path_str
            );
            // RISK (Low): Delete A -> Create New A quickly may rebind.
            self.repo.bind_inode(&inode, existing_id)?;
            // 内容更新记录为 pending（不自动 reconcile）
            self.upsert_pending(path_str, ChangeStatus::Modified)?;
            let mut msgs = self.gen_list()?;
            msgs.push(self.fs_change_msg(path_str, "modified"));
            return Ok(msgs);
        }

        // 3b. Check Content for UUID (Recovery)
        let content = std::fs::read_to_string(&file_path)?;
        if let Some(recovered_id) = recovery::try_recover_from_content(&content)
            && let Ok(Some(old_path)) = self.repo.get_path_by_docid(recovered_id)
        {
            info!(
                "Handler: Recovery UUID found. Resurrecting {:?} from {} to {}",
                recovered_id, old_path, path_str
            );
            if old_path != path_str {
                self.repo.rename_doc(&old_path, path_str)?;
            }
            self.repo.bind_inode(&inode, recovered_id)?;
            self.upsert_pending(path_str, ChangeStatus::Modified)?;
            let mut msgs = self.gen_list()?;
            msgs.push(self.fs_change_msg(path_str, "modified"));
            return Ok(msgs);
        }

        // 3c. Truly New File — 创建元数据，但内容不 ingest 到 Ledger
        info!("Handler: New file detected: {}", path_str);
        let doc_id = self.repo.create_docid(path_str)?;
        self.repo.bind_inode(&inode, doc_id)?;
        self.upsert_pending(path_str, ChangeStatus::Added)?;
        let mut msgs = self.gen_list()?;
        msgs.push(self.fs_change_msg(path_str, "added"));
        Ok(msgs)
    }

    fn gen_list(&self) -> Result<Vec<ServerMessage>> {
        let docs = self.repo.list_docs(&RepoType::Local(uuid::Uuid::nil()))?;
        Ok(vec![ServerMessage::DocList { docs }])
    }

    /// 将文件变更记录到 pending_fs_ops 表
    fn upsert_pending(&self, path_str: &str, status: ChangeStatus) -> Result<()> {
        let hash = if status == ChangeStatus::Deleted {
            String::new()
        } else {
            let file_path = self.vault_root.join(path_str);
            let content = std::fs::read_to_string(&file_path).unwrap_or_default();
            pending_fs::content_hash(&content)
        };
        let entry = PendingFsEntry {
            path: path_str.to_string(),
            change_type: status,
            content_hash: hash,
            detected_at: chrono::Utc::now().timestamp_millis(),
        };
        pending_fs::upsert(&self.repo.local_db, &entry)
    }

    /// 构造 FsChangeDetected 消息
    fn fs_change_msg(&self, path: &str, change_type: &str) -> ServerMessage {
        ServerMessage::FsChangeDetected {
            path: path.to_string(),
            change_type: change_type.to_string(),
        }
    }
}
