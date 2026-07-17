//! # 文件系统事件处理器 (FS Event Handler)
//! plan_ref:
//!   - 03_storage/watcher#watcher-contract
//!   - 05_diff_logic#source-control-runtime
//!
//! Invariants:
//! - Watcher 传入的路径必须先被解析为 `(repo_name, repo_id, repo_path)`。
//! - 外部文件事件只能更新 pending side table；不得提前改写权威 Ledger / Path 映射。

use crate::ledger::RepoManager;
use crate::source_control::ChangeStatus;
use crate::sync::watcher::{WatcherRefresh, WatcherRefreshKind};
use crate::utils::fs::checked_exists;
use crate::vfs::Vfs;
use anyhow::Result;
use std::sync::Arc;
use tracing::{info, warn};

use super::pending_content::PendingSyncResult;

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

    pub fn handle_event(&self, repo_path: &str) -> Result<Vec<WatcherRefresh>> {
        let file_path = self
            .repo
            .local_repo_workspace_path(self.repo_name, repo_path)?;

        if !checked_exists(&file_path, "workspace path while handling fs event")? {
            return self.handle_delete(repo_path);
        }

        let inode = match self.vfs.get_inode_abs(&file_path)? {
            Some(inode) => inode,
            None => return self.handle_delete(repo_path),
        };

        if let Some(doc_id) = self
            .repo
            .get_docid_by_inode_in_local_repo(self.repo_name, &inode)?
        {
            return self.handle_known_inode(repo_path, doc_id);
        }

        if let Some(existing_id) = self
            .repo
            .get_tracked_docid_in_local_repo(self.repo_name, repo_path)?
        {
            info!("Handler: Inode change (Atomic Save?) for {}", repo_path);
            self.repo
                .bind_inode_in_local_repo(self.repo_name, &inode, existing_id)?;
            return match super::pending_content::sync_modified_pending(
                self.repo,
                self.repo_name,
                repo_path,
                existing_id,
            )? {
                PendingSyncResult::Changed => self.modified_refresh(repo_path),
                PendingSyncResult::Noop => Ok(vec![]),
            };
        }

        let changed = super::pending::upsert(
            self.repo,
            self.repo_name,
            repo_path,
            ChangeStatus::Added,
            None,
            None,
        )?;
        if !changed {
            return Ok(vec![]);
        }
        info!("Handler: New file detected: {}", repo_path);
        Ok(vec![super::pending::refresh(
            self.repo,
            self.repo_name,
            self.repo_id,
            repo_path,
            WatcherRefreshKind::Added,
        )?])
    }

    fn handle_delete(&self, repo_path: &str) -> Result<Vec<WatcherRefresh>> {
        let had_pending_add =
            super::pending::has_pending_added(self.repo, self.repo_name, repo_path)?;
        let tracked_doc_id = self
            .repo
            .resolve_canonical_doc_id_in_local_repo(self.repo_name, repo_path)?;
        let has_tracked_doc = tracked_doc_id.is_some();

        if !has_tracked_doc && !had_pending_add {
            return Ok(vec![]);
        }
        if had_pending_add && !has_tracked_doc {
            super::pending::clear(self.repo, self.repo_name, repo_path)?;
            return self.modified_refresh(repo_path);
        }

        let changed = super::pending::upsert(
            self.repo,
            self.repo_name,
            repo_path,
            ChangeStatus::Deleted,
            tracked_doc_id,
            None,
        )?;
        if !changed {
            return Ok(vec![]);
        }
        warn!(
            "Handler: File gone: {}. Recording as pending delete.",
            repo_path
        );
        Ok(vec![super::pending::refresh(
            self.repo,
            self.repo_name,
            self.repo_id,
            repo_path,
            WatcherRefreshKind::Deleted,
        )?])
    }

    fn handle_known_inode(
        &self,
        repo_path: &str,
        doc_id: crate::models::DocId,
    ) -> Result<Vec<WatcherRefresh>> {
        if let Some(meta) = self
            .repo
            .get_file_meta_for_doc_in_local_repo(self.repo_name, doc_id)?
            && meta.path != repo_path
        {
            return self.record_external_rename(&meta.path, repo_path, doc_id);
        }

        match super::pending_content::sync_modified_pending(
            self.repo,
            self.repo_name,
            repo_path,
            doc_id,
        )? {
            PendingSyncResult::Changed => {
                info!("Handler: Content update detected for {}", repo_path);
                self.modified_refresh(repo_path)
            }
            PendingSyncResult::Noop => Ok(vec![]),
        }
    }

    fn modified_refresh(&self, repo_path: &str) -> Result<Vec<WatcherRefresh>> {
        Ok(vec![super::pending::refresh(
            self.repo,
            self.repo_name,
            self.repo_id,
            repo_path,
            WatcherRefreshKind::Modified,
        )?])
    }

    fn record_external_rename(
        &self,
        old_path: &str,
        new_path: &str,
        doc_id: crate::models::DocId,
    ) -> Result<Vec<WatcherRefresh>> {
        let changed = super::pending_rename::upsert_external_rename(
            self.repo,
            self.repo_name,
            old_path,
            new_path,
            doc_id,
        )?;
        if !changed {
            return Ok(vec![]);
        }
        info!("Handler: Rename detected {} -> {}", old_path, new_path);
        Ok(vec![
            super::pending::refresh(
                self.repo,
                self.repo_name,
                self.repo_id,
                old_path,
                WatcherRefreshKind::Deleted,
            )?,
            super::pending::refresh(
                self.repo,
                self.repo_name,
                self.repo_id,
                new_path,
                WatcherRefreshKind::Added,
            )?,
        ])
    }
}
