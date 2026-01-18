// crates/core/src/ledger/manager/source_control_ops.rs
//! # 版本控制集成
//!
//! 实现 `RepoManager` 的暂存、提交、历史等版本控制方法。

use crate::ledger::RepoManager;
use crate::ledger::source_control;
use crate::models::DocId;
use crate::source_control::{ChangeEntry, ChangeStatus, CommitInfo};
use anyhow::Result;

impl RepoManager {
    /// 暂存指定文件
    pub fn stage_file(&self, path: &str) -> Result<()> {
        source_control::stage_file(&self.local_db, path)
    }

    /// 取消暂存指定文件
    pub fn unstage_file(&self, path: &str) -> Result<()> {
        source_control::unstage_file(&self.local_db, path)
    }

    /// 获取已暂存文件列表
    pub fn list_staged(&self) -> Result<Vec<ChangeEntry>> {
        source_control::list_staged(&self.local_db)
    }

    /// 创建提交 (保存快照)
    pub fn create_commit_with_snapshots<F>(
        &self,
        message: &str,
        get_content: F,
    ) -> Result<CommitInfo>
    where
        F: Fn(&str) -> Option<(DocId, String)>,
    {
        source_control::create_commit(&self.local_db, message, get_content)
    }

    /// 获取提交历史
    pub fn list_commits(&self, limit: u32) -> Result<Vec<CommitInfo>> {
        source_control::list_commits(&self.local_db, limit)
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
}
