// crates/core/src/ledger/manager/source_control_api.rs
//! plan_ref:
//!   - 07_diff_logic#source-control-runtime
//!   - 06_repository#repo-selector-resolution-contract
//!
//! # Source Control API 实现 (RepoManager)

use crate::ledger::RepoManager;
use crate::ledger::traits::RepoSelector;
use crate::protocol::ScPathTarget;
use crate::source_control::{ChangeEntry, CommitFileDiff, CommitInfo, SourceControlApi};
use anyhow::Result;

impl SourceControlApi for RepoManager {
    fn list_pending_fs_in_repo(&self, repo: &RepoSelector) -> Result<Vec<ChangeEntry>> {
        let repo_name = self.resolve_local_repo_selector_for_execution(repo)?;
        self.list_pending_fs_in_local_repo(&repo_name)
    }

    fn list_staged_in_repo(&self, repo: &RepoSelector) -> Result<Vec<ChangeEntry>> {
        let repo_name = self.resolve_local_repo_selector_for_execution(repo)?;
        self.list_staged_in_local_repo(&repo_name)
    }

    fn stage_pending_in_repo(&self, repo: &RepoSelector, target: &ScPathTarget) -> Result<()> {
        let repo_name = self.resolve_local_repo_selector_for_execution(repo)?;
        self.stage_pending_target_in_local_repo(&repo_name, target)
    }

    fn discard_pending_in_repo(&self, repo: &RepoSelector, target: &ScPathTarget) -> Result<()> {
        let repo_name = self.resolve_local_repo_selector_for_execution(repo)?;
        self.discard_pending_target_in_local_repo(&repo_name, target)
    }

    fn unstage_file_in_repo(&self, repo: &RepoSelector, target: &ScPathTarget) -> Result<()> {
        let repo_name = self.resolve_local_repo_selector_for_execution(repo)?;
        self.unstage_file_target_in_local_repo(&repo_name, target)
    }

    fn list_changes_in_repo(&self, repo: &RepoSelector) -> Result<Vec<ChangeEntry>> {
        let repo_name = self.resolve_local_repo_selector_for_execution(repo)?;
        self.list_changes_in_local_repo(&repo_name)
    }

    fn diff_doc_path_in_repo(&self, repo: &RepoSelector, target: &ScPathTarget) -> Result<String> {
        let repo_name = self.resolve_local_repo_selector_for_execution(repo)?;
        self.diff_doc_target_in_local_repo(&repo_name, target)
    }

    fn list_commits_in_repo(&self, repo: &RepoSelector, limit: u32) -> Result<Vec<CommitInfo>> {
        let repo_name = self.resolve_local_repo_selector_for_execution(repo)?;
        self.list_commits_in_local_repo(&repo_name, limit)
    }

    fn diff_commits_in_repo(
        &self,
        repo: &RepoSelector,
        commit_a_id: Option<&str>,
        commit_b_id: &str,
    ) -> Result<Vec<CommitFileDiff>> {
        let repo_name = self.resolve_local_repo_selector_for_execution(repo)?;
        self.diff_commits_in_local_repo(&repo_name, commit_a_id, commit_b_id)
    }

    fn commit_staged_in_repo(&self, repo: &RepoSelector, message: &str) -> Result<CommitInfo> {
        let repo_name = self.resolve_local_repo_selector_for_execution(repo)?;
        self.commit_staged_in_local_repo(&repo_name, message)
    }
}
