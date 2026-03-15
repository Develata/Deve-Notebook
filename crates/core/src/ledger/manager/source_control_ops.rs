// crates/core/src/ledger/manager/source_control_ops.rs
//! # 版本控制集成
//!
//! 实现 `RepoManager` 的暂存、提交、历史等版本控制方法。

use crate::ledger::RepoManager;
use crate::ledger::source_control;
use crate::models::DocId;
use crate::source_control::{ChangeEntry, ChangeStatus, CommitInfo, pending_fs};
use anyhow::Result;

impl RepoManager {
    /// 取消暂存指定文件
    pub fn unstage_file(&self, path: &str) -> Result<()> {
        self.unstage_file_in_local_repo(self.local_repo_name(), path)
    }

    pub fn unstage_file_in_local_repo(&self, repo_name: &str, path: &str) -> Result<()> {
        let target = self.tracked_target_for_path_in_local_repo(repo_name, path)?;
        self.unstage_file_target_in_local_repo(repo_name, &target)
    }

    /// 获取已暂存文件列表
    pub fn list_staged(&self) -> Result<Vec<ChangeEntry>> {
        self.list_staged_in_local_repo(self.local_repo_name())
    }

    pub fn list_staged_in_local_repo(&self, repo_name: &str) -> Result<Vec<ChangeEntry>> {
        self.run_on_local_repo(repo_name, source_control::list_staged)
    }

    /// 获取提交历史
    pub fn list_commits(&self, limit: u32) -> Result<Vec<CommitInfo>> {
        self.list_commits_in_local_repo(self.local_repo_name(), limit)
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

    /// 提交已暂存文件（三阶段工作流的唯一入口）
    ///
    /// **流程**: 读取暂存文件 → 读磁盘内容 → diff 快照 → 生成 Op → 追加 Ledger → 快照更新 → 创建提交
    /// **Invariant**: `vault_root` 必须存在；不存在即为配置错误，而不是兼容回退场景。
    pub fn commit_staged(&self, message: &str) -> Result<CommitInfo> {
        self.commit_staged_in_local_repo(self.local_repo_name(), message)
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

    /// 获取所有待确认的文件变更
    pub fn list_pending_fs(&self) -> Result<Vec<ChangeEntry>> {
        self.list_pending_fs_in_local_repo(self.local_repo_name())
    }

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

    /// 将待确认变更移入暂存区 (Working Dir → Staging)
    ///
    /// **流程**: 读取 pending 状态 → pending_fs_ops 中移除 → staged_files 中插入（带状态）
    pub fn stage_pending(&self, path: &str) -> Result<()> {
        self.stage_pending_in_local_repo(self.local_repo_name(), path)
    }

    pub fn stage_pending_in_local_repo(&self, repo_name: &str, path: &str) -> Result<()> {
        let target = self.tracked_target_for_path_in_local_repo(repo_name, path)?;
        self.stage_pending_target_in_local_repo(repo_name, &target)
    }

    /// 丢弃待确认变更
    pub fn discard_pending(&self, path: &str) -> Result<()> {
        self.discard_pending_in_local_repo(self.local_repo_name(), path)
    }

    pub fn discard_pending_in_local_repo(&self, repo_name: &str, path: &str) -> Result<()> {
        let target = self.tracked_target_for_path_in_local_repo(repo_name, path)?;
        self.discard_pending_target_in_local_repo(repo_name, &target)
            .map(|_| ())
    }
}
