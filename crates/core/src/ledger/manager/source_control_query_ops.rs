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
use crate::source_control::{ChangeEntry, CommitFileDiff};
use anyhow::Result;
use std::collections::HashSet;

use super::source_control_target_resolution::change_identity_key;

impl RepoManager {
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
