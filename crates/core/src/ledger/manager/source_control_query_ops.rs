//! plan_ref:
//!   - 05_diff_logic#source-control-runtime
//!   - 04_repository#repo-selector-resolution-contract
//!   - 03_storage/index#internal-path-normalization
//!
//! # 版本控制查询
//!
//! Invariants:
//! - 工作区差异列表以 `pending_fs_ops + staging` 为准，而不是重新推断第二真值。
//! - Diff 左侧来自当前 Ledger 投影，右侧来自工作区文件。

use crate::ledger::RepoManager;
use crate::models::DocId;
use crate::protocol::ScPathTarget;
use crate::source_control::{ChangeEntry, ChangeStatus, CommitFileDiff, CommitInfo};
use anyhow::Result;

impl RepoManager {
    pub fn list_staged_in_local_repo(&self, repo_name: &str) -> Result<Vec<ChangeEntry>> {
        self.source_control_runtime()
            .list_staged_in_local_repo(repo_name)
    }

    pub fn list_pending_fs_in_local_repo(&self, repo_name: &str) -> Result<Vec<ChangeEntry>> {
        self.source_control_runtime()
            .list_pending_fs_in_local_repo(repo_name)
    }

    pub fn list_confirmed_ledger_changes_in_local_repo(
        &self,
        repo_name: &str,
    ) -> Result<Vec<ChangeEntry>> {
        self.source_control_runtime()
            .list_confirmed_ledger_in_local_repo(repo_name)
    }

    pub fn list_changes(&self) -> Result<Vec<ChangeEntry>> {
        self.list_changes_in_local_repo(self.local_repo_name())
    }

    pub fn list_changes_in_local_repo(&self, repo_name: &str) -> Result<Vec<ChangeEntry>> {
        self.source_control_runtime()
            .list_changes_in_local_repo(repo_name)
    }

    pub fn diff_doc_path(&self, path: &str) -> Result<String> {
        self.diff_doc_path_in_local_repo(self.local_repo_name(), path)
    }

    pub fn diff_doc_path_in_local_repo(&self, repo_name: &str, path: &str) -> Result<String> {
        self.source_control_runtime()
            .diff_doc_path_in_local_repo(repo_name, path)
    }

    pub fn diff_doc_target_in_local_repo(
        &self,
        repo_name: &str,
        target: &ScPathTarget,
    ) -> Result<String> {
        self.source_control_runtime()
            .diff_doc_target_in_local_repo(repo_name, target)
    }

    /// 获取文档的已提交内容 (用于 Diff)
    pub fn get_committed_content(&self, doc_id: DocId) -> Result<Option<String>> {
        self.source_control_runtime().get_committed_content(doc_id)
    }

    /// 检测单个文档的变更状态
    pub fn detect_change(
        &self,
        committed: Option<&str>,
        current: Option<&str>,
    ) -> Option<ChangeStatus> {
        self.source_control_runtime()
            .detect_change(committed, current)
    }

    pub fn list_commits_in_local_repo(
        &self,
        repo_name: &str,
        limit: u32,
    ) -> Result<Vec<CommitInfo>> {
        self.source_control_runtime()
            .list_commits_in_local_repo(repo_name, limit)
    }

    pub fn diff_commits(
        &self,
        commit_a_id: Option<&str>,
        commit_b_id: &str,
    ) -> Result<Vec<CommitFileDiff>> {
        self.diff_commits_in_local_repo(self.local_repo_name(), commit_a_id, commit_b_id)
    }

    pub fn diff_commits_in_local_repo(
        &self,
        repo_name: &str,
        commit_a_id: Option<&str>,
        commit_b_id: &str,
    ) -> Result<Vec<CommitFileDiff>> {
        self.source_control_runtime().diff_commits_in_local_repo(
            repo_name,
            commit_a_id,
            commit_b_id,
        )
    }
}
