// crates/core/src/ledger/manager/source_control_api.rs
//! plan_ref:
//!   - 05_diff_logic#source-control-runtime
//!   - 04_repository#repo-selector-resolution-contract
//!
//! # Source Control API 实现 (RepoManager)

use crate::ledger::RepoManager;
use crate::ledger::traits::RepoSelector;
use crate::protocol::ScPathTarget;
use crate::source_control::{ChangeEntry, CommitFileDiff, CommitInfo, SourceControlApi};
use anyhow::Result;

impl SourceControlApi for RepoManager {
    fn list_pending_fs_in_repo(&self, repo: &RepoSelector) -> Result<Vec<ChangeEntry>> {
        self.source_control_scoped_runtime()
            .list_pending_fs_in_repo(repo)
    }

    fn list_staged_in_repo(&self, repo: &RepoSelector) -> Result<Vec<ChangeEntry>> {
        self.source_control_scoped_runtime()
            .list_staged_in_repo(repo)
    }

    fn stage_pending_in_repo(&self, repo: &RepoSelector, target: &ScPathTarget) -> Result<()> {
        self.source_control_scoped_runtime()
            .stage_pending_in_repo(repo, target)
    }

    fn discard_pending_in_repo(&self, repo: &RepoSelector, target: &ScPathTarget) -> Result<()> {
        self.source_control_scoped_runtime()
            .discard_pending_in_repo(repo, target)
    }

    fn unstage_file_in_repo(&self, repo: &RepoSelector, target: &ScPathTarget) -> Result<()> {
        self.source_control_scoped_runtime()
            .unstage_file_in_repo(repo, target)
    }

    fn list_changes_in_repo(&self, repo: &RepoSelector) -> Result<Vec<ChangeEntry>> {
        self.source_control_scoped_runtime()
            .list_changes_in_repo(repo)
    }

    fn diff_doc_path_in_repo(&self, repo: &RepoSelector, target: &ScPathTarget) -> Result<String> {
        self.source_control_scoped_runtime()
            .diff_doc_path_in_repo(repo, target)
    }

    fn list_commits_in_repo(&self, repo: &RepoSelector, limit: u32) -> Result<Vec<CommitInfo>> {
        self.source_control_scoped_runtime()
            .list_commits_in_repo(repo, limit)
    }

    fn diff_commits_in_repo(
        &self,
        repo: &RepoSelector,
        commit_a_id: Option<&str>,
        commit_b_id: &str,
    ) -> Result<Vec<CommitFileDiff>> {
        self.source_control_scoped_runtime()
            .diff_commits_in_repo(repo, commit_a_id, commit_b_id)
    }

    fn commit_staged_in_repo(&self, repo: &RepoSelector, message: &str) -> Result<CommitInfo> {
        self.source_control_scoped_runtime()
            .commit_staged_in_repo(repo, message)
    }
}
