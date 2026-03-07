//! # 文件系统事件处理器 (FS Event Handler)
//!
//! Invariants:
//! - Watcher 传入的路径必须先被解析为 `(repo_name, repo_id, repo_path)`。
//! - 本地文件事件只能修改对应 repo 的 metadata / pending / tree 状态。

use super::rebuild;
use crate::ledger::RepoManager;
use crate::protocol::ServerMessage;
use crate::source_control::ChangeStatus;
use crate::source_control::pending_fs::{self, PendingFsEntry};
use crate::sync::recovery;
use crate::vfs::Vfs;
use anyhow::Result;
use std::sync::Arc;
use tracing::{info, warn};

pub struct FsEventHandler<'a> {
    repo: &'a Arc<RepoManager>,
    vfs: &'a Vfs,
    repo_name: &'a str,
    repo_id: crate::models::RepoId,
}

impl<'a> FsEventHandler<'a> {
    pub fn new(
        repo: &'a Arc<RepoManager>,
        vfs: &'a Vfs,
        repo_name: &'a str,
        repo_id: crate::models::RepoId,
    ) -> Self {
        Self {
            repo,
            vfs,
            repo_name,
            repo_id,
        }
    }

    pub fn handle_event(&self, repo_path: &str) -> Result<Vec<ServerMessage>> {
        let file_path = self
            .repo
            .local_repo_workspace_path(self.repo_name, repo_path)?;

        if !file_path.exists() {
            return self.handle_delete(repo_path);
        }

        let root_rel = self
            .repo
            .local_repo_workspace_relative(self.repo_name, repo_path);
        let inode = match self.vfs.get_inode(&root_rel)? {
            Some(inode) => inode,
            None => return Ok(vec![]),
        };

        if let Some(doc_id) = self
            .repo
            .get_docid_by_inode_in_local_repo(self.repo_name, &inode)?
        {
            return self.handle_known_inode(repo_path, doc_id);
        }

        if let Some(existing_id) = self
            .repo
            .get_docid_in_local_repo(self.repo_name, repo_path)?
        {
            info!("Handler: Inode change (Atomic Save?) for {}", repo_path);
            self.repo
                .bind_inode_in_local_repo(self.repo_name, &inode, existing_id)?;
            self.sync_modified_pending(repo_path, existing_id)?;
            return self.modified_refresh(repo_path);
        }

        let content = std::fs::read_to_string(&file_path)?;
        if let Some(recovered_id) = recovery::try_recover_from_content(&content)
            && let Some(old_path) = self
                .repo
                .get_path_by_docid_in_local_repo(self.repo_name, recovered_id)?
        {
            info!(
                "Handler: Recovery UUID found. Resurrecting {:?} from {} to {}",
                recovered_id, old_path, repo_path
            );
            if old_path != repo_path {
                self.repo
                    .rename_doc_in_local_repo(self.repo_name, &old_path, repo_path)?;
            }
            self.repo
                .bind_inode_in_local_repo(self.repo_name, &inode, recovered_id)?;
            self.sync_modified_pending(repo_path, recovered_id)?;
            return self.modified_refresh(repo_path);
        }

        info!("Handler: New file detected: {}", repo_path);
        let doc_id = self
            .repo
            .create_docid_in_local_repo(self.repo_name, repo_path)?;
        self.repo
            .bind_inode_in_local_repo(self.repo_name, &inode, doc_id)?;
        self.upsert_pending(repo_path, ChangeStatus::Added)?;
        let mut msgs = self.gen_list()?;
        msgs.push(self.fs_change_msg(repo_path, "added"));
        Ok(msgs)
    }

    fn handle_delete(&self, repo_path: &str) -> Result<Vec<ServerMessage>> {
        if self
            .repo
            .get_docid_in_local_repo(self.repo_name, repo_path)?
            .is_none()
        {
            return Ok(vec![]);
        }

        warn!(
            "Handler: File gone: {}. Recording as pending delete.",
            repo_path
        );
        self.repo
            .delete_doc_in_local_repo(self.repo_name, repo_path)?;
        self.upsert_pending(repo_path, ChangeStatus::Deleted)?;
        let mut msgs = self.gen_list()?;
        msgs.push(self.fs_change_msg(repo_path, "deleted"));
        Ok(msgs)
    }

    fn handle_known_inode(
        &self,
        repo_path: &str,
        doc_id: crate::models::DocId,
    ) -> Result<Vec<ServerMessage>> {
        if let Some(known_path) = self
            .repo
            .get_path_by_docid_in_local_repo(self.repo_name, doc_id)?
            && known_path != repo_path
        {
            info!("Handler: Rename detected {} -> {}", known_path, repo_path);
            self.repo
                .rename_doc_in_local_repo(self.repo_name, &known_path, repo_path)?;
            return self.gen_list();
        }

        info!("Handler: Content update detected for {}", repo_path);
        self.sync_modified_pending(repo_path, doc_id)?;
        self.modified_refresh(repo_path)
    }

    fn modified_refresh(&self, repo_path: &str) -> Result<Vec<ServerMessage>> {
        let mut msgs = self.gen_list()?;
        msgs.push(self.fs_change_msg(repo_path, "modified"));
        Ok(msgs)
    }

    fn gen_list(&self) -> Result<Vec<ServerMessage>> {
        let docs = self.repo.list_local_docs(Some(self.repo_name))?;
        Ok(vec![ServerMessage::DocList { docs }])
    }

    fn sync_modified_pending(&self, repo_path: &str, doc_id: crate::models::DocId) -> Result<()> {
        let file_path = self
            .repo
            .local_repo_workspace_path(self.repo_name, repo_path)?;
        let disk_content = std::fs::read_to_string(&file_path).unwrap_or_default();
        let rebuilt = rebuild::rebuild_local_doc_in_repo(self.repo, self.repo_name, doc_id)?;
        if rebuilt.content == disk_content {
            return self.repo.run_on_local_repo(self.repo_name, |db| {
                let _ = pending_fs::remove(db, repo_path);
                Ok(())
            });
        }
        self.upsert_pending(repo_path, ChangeStatus::Modified)
    }

    fn upsert_pending(&self, repo_path: &str, status: ChangeStatus) -> Result<()> {
        let hash = if status == ChangeStatus::Deleted {
            String::new()
        } else {
            let file_path = self
                .repo
                .local_repo_workspace_path(self.repo_name, repo_path)?;
            let content = std::fs::read_to_string(&file_path).unwrap_or_default();
            pending_fs::content_hash(&content)
        };

        let has_conflict = match self
            .repo
            .get_docid_in_local_repo(self.repo_name, repo_path)?
        {
            Some(doc_id) => self.repo.run_on_local_repo(self.repo_name, |db| {
                crate::source_control::conflict::check_conflict(db, doc_id, &hash)
            })?,
            None => false,
        };

        let entry = PendingFsEntry {
            path: repo_path.to_string(),
            change_type: status,
            content_hash: hash,
            detected_at: chrono::Utc::now().timestamp_millis(),
            has_conflict,
        };
        self.repo
            .run_on_local_repo(self.repo_name, |db| pending_fs::upsert(db, &entry))
    }

    fn fs_change_msg(&self, path: &str, change_type: &str) -> ServerMessage {
        let has_conflict = self
            .repo
            .run_on_local_repo(self.repo_name, |db| {
                Ok(pending_fs::get(db, path)?
                    .map(|e| e.has_conflict)
                    .unwrap_or(false))
            })
            .unwrap_or(false);
        ServerMessage::FsChangeDetected {
            repo_id: Some(self.repo_id),
            path: path.to_string(),
            change_type: change_type.to_string(),
            has_conflict,
        }
    }
}
