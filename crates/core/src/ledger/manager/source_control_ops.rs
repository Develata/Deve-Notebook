// crates/core/src/ledger/manager/source_control_ops.rs
//! plan_ref:
//!   - 07_diff_logic#source-control-runtime
//!   - 06_repository#repo-selector-resolution-contract
//!   - 04_storage#watcher-contract
//!
//! # 版本控制集成
//!
//! 实现 `RepoManager` 的 repo-scoped 暂存、提交、历史等版本控制方法。

use crate::ledger::RepoManager;
use crate::ledger::source_control;
use crate::models::DocId;
use crate::source_control::{ChangeEntry, ChangeStatus, CommitInfo, pending_fs};
use anyhow::Result;

impl RepoManager {
    pub fn unstage_file_in_local_repo(&self, repo_name: &str, path: &str) -> Result<()> {
        let target = self.tracked_target_for_path_in_local_repo(repo_name, path)?;
        self.unstage_file_target_in_local_repo(repo_name, &target)
    }

    pub fn list_staged_in_local_repo(&self, repo_name: &str) -> Result<Vec<ChangeEntry>> {
        self.run_on_local_repo(repo_name, source_control::list_staged)
    }

    pub fn list_commits_in_local_repo(
        &self,
        repo_name: &str,
        limit: u32,
    ) -> Result<Vec<CommitInfo>> {
        self.run_on_local_repo(repo_name, |db| source_control::list_commits(db, limit))
    }

    /// 获取文档的已提交内容 (用于 Diff)
    pub fn get_committed_content(&self, doc_id: DocId) -> Result<Option<String>> {
        source_control::validate_tables(self.local_db.as_ref()).map_err(|err| {
            anyhow::anyhow!(
                "Broken local repo {} while validating source control tables: {}",
                self.local_repo_name,
                err
            )
        })?;
        source_control::get_committed_content(&self.local_db, doc_id)
    }

    /// 检测单个文档的变更状态
    pub fn detect_change(
        &self,
        committed: Option<&str>,
        current: Option<&str>,
    ) -> Option<ChangeStatus> {
        source_control::detect_change(committed, current)
    }

    pub fn commit_staged_in_local_repo(
        &self,
        repo_name: &str,
        message: &str,
    ) -> Result<CommitInfo> {
        let Some(vault_root) = &self.vault_root else {
            anyhow::bail!("vault_root is required for staged commits");
        };
        if repo_name == self.local_repo_name() {
            return self.commit_staged_with_ops(message, vault_root.clone());
        }
        self.commit_staged_with_ops_in_local_repo(repo_name, message, vault_root.clone())
    }

    // === Pending FS Ops (Working Directory) ===

    pub fn list_pending_fs_in_local_repo(&self, repo_name: &str) -> Result<Vec<ChangeEntry>> {
        self.run_on_local_repo(repo_name, |db| {
            let entries = pending_fs::list_all(db)?;
            Ok(entries
                .into_iter()
                .map(|e| ChangeEntry {
                    path: e.path,
                    renamed_from: e.renamed_from,
                    doc_id: e.doc_id,
                    status: e.change_type,
                    has_conflict: e.has_conflict,
                })
                .collect())
        })
    }

    pub fn stage_pending_in_local_repo(&self, repo_name: &str, path: &str) -> Result<()> {
        let target = self.tracked_target_for_path_in_local_repo(repo_name, path)?;
        self.stage_pending_target_in_local_repo(repo_name, &target)
    }

    pub fn discard_pending_in_local_repo(&self, repo_name: &str, path: &str) -> Result<()> {
        let target = self.tracked_target_for_path_in_local_repo(repo_name, path)?;
        self.discard_pending_target_in_local_repo(repo_name, &target)
            .map(|_| ())
    }
}
