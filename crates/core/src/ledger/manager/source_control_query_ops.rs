//! # 版本控制查询
//!
//! Invariants:
//! - 工作区差异列表以 `pending_fs_ops + staging` 为准，而不是重新推断第二真值。
//! - Diff 左侧来自当前 Ledger 投影，右侧来自工作区文件。

use crate::ledger::RepoManager;
use crate::source_control::diff;
use crate::source_control::{ChangeEntry, CommitFileDiff};
use crate::utils::path::to_forward_slash;
use anyhow::Result;
use std::collections::HashSet;

impl RepoManager {
    pub fn list_changes(&self) -> Result<Vec<ChangeEntry>> {
        self.list_changes_in_local_repo(self.local_repo_name())
    }

    pub fn list_changes_in_local_repo(&self, repo_name: &str) -> Result<Vec<ChangeEntry>> {
        let staged = self.list_staged_in_local_repo(repo_name)?;
        let staged_paths: HashSet<String> = staged
            .iter()
            .map(|entry| to_forward_slash(&entry.path))
            .collect();
        let mut changes = staged;
        changes.extend(
            self.list_pending_fs_in_local_repo(repo_name)?
                .into_iter()
                .filter(|entry| !staged_paths.contains(&to_forward_slash(&entry.path))),
        );
        Ok(changes)
    }

    pub fn diff_doc_path(&self, path: &str) -> Result<String> {
        self.diff_doc_path_in_local_repo(self.local_repo_name(), path)
    }

    pub fn diff_doc_path_in_local_repo(&self, repo_name: &str, path: &str) -> Result<String> {
        let normalized = to_forward_slash(path);
        let (old_content, new_content) =
            self.workdir_diff_inputs_in_local_repo(repo_name, &normalized)?;
        Ok(diff::unified_diff(&old_content, &new_content, &normalized))
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
