//! plan_ref:
//!   - 07_diff_logic#source-control-runtime
//!   - 06_repository#repo-selector-resolution-contract
//!   - 04_storage#internal-path-normalization
//!
//! # 版本控制查询
//!
//! Invariants:
//! - 工作区差异列表以 `pending_fs_ops + staging` 为准，而不是重新推断第二真值。
//! - Diff 左侧来自当前 Ledger 投影，右侧来自工作区文件。

use crate::ledger::RepoManager;
use crate::ledger::source_control;
use crate::models::DocId;
use crate::protocol::ScPathTarget;
use crate::source_control::{
    ChangeEntry, ChangeStatus, CommitFileDiff, CommitInfo, diff, pending_fs,
};
use anyhow::Result;
use std::collections::HashSet;

use super::source_control_target_resolution::change_identity_key;

impl RepoManager {
    pub fn list_staged_in_local_repo(&self, repo_name: &str) -> Result<Vec<ChangeEntry>> {
        self.run_on_local_repo(repo_name, source_control::list_staged)
    }

    pub fn list_pending_fs_in_local_repo(&self, repo_name: &str) -> Result<Vec<ChangeEntry>> {
        self.run_on_local_repo(repo_name, |db| {
            let entries = pending_fs::list_all(db)?;
            Ok(entries
                .into_iter()
                .map(|entry| ChangeEntry {
                    path: entry.path,
                    renamed_from: entry.renamed_from,
                    doc_id: entry.doc_id,
                    status: entry.change_type,
                    has_conflict: entry.has_conflict,
                })
                .collect())
        })
    }

    pub fn list_changes(&self) -> Result<Vec<ChangeEntry>> {
        self.list_changes_in_local_repo(self.local_repo_name())
    }

    pub fn list_changes_in_local_repo(&self, repo_name: &str) -> Result<Vec<ChangeEntry>> {
        let staged = self.list_staged_in_local_repo(repo_name)?;
        let staged_keys: HashSet<_> = staged.iter().map(change_identity_key).collect();
        let mut changes = staged;
        changes.extend(
            self.list_pending_fs_in_local_repo(repo_name)?
                .into_iter()
                .filter(|entry| !staged_keys.contains(&change_identity_key(entry))),
        );
        Ok(changes)
    }

    pub fn diff_doc_path(&self, path: &str) -> Result<String> {
        self.diff_doc_path_in_local_repo(self.local_repo_name(), path)
    }

    pub fn diff_doc_path_in_local_repo(&self, repo_name: &str, path: &str) -> Result<String> {
        let target = self.tracked_target_for_path_in_local_repo(repo_name, path)?;
        self.diff_doc_target_in_local_repo(repo_name, &target)
    }

    pub fn diff_doc_target_in_local_repo(
        &self,
        repo_name: &str,
        target: &ScPathTarget,
    ) -> Result<String> {
        let (path, old_content, new_content) =
            self.workdir_diff_inputs_for_target_in_local_repo(repo_name, target)?;
        Ok(diff::unified_diff(&old_content, &new_content, &path))
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

    pub fn list_commits_in_local_repo(
        &self,
        repo_name: &str,
        limit: u32,
    ) -> Result<Vec<CommitInfo>> {
        self.run_on_local_repo(repo_name, |db| source_control::list_commits(db, limit))
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
        let commit_a = commit_a_id.map(str::to_owned);
        let commit_b = commit_b_id.to_owned();
        self.run_on_local_repo(repo_name, |db| {
            crate::source_control::commit_diff::compare_commits(db, commit_a.as_deref(), &commit_b)
        })
    }
}
