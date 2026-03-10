// crates/core/src/source_control/api.rs
//! # Source Control API (Trait)

use crate::ledger::traits::RepoSelector;
use crate::protocol::ScPathTarget;
use crate::source_control::{ChangeEntry, CommitFileDiff, CommitInfo};
use anyhow::Result;

pub trait SourceControlApi: Send + Sync {
    fn list_pending_fs_in_repo(&self, repo: &RepoSelector) -> Result<Vec<ChangeEntry>>;
    fn stage_pending_in_repo(&self, repo: &RepoSelector, target: &ScPathTarget) -> Result<()>;
    fn discard_pending_in_repo(&self, repo: &RepoSelector, target: &ScPathTarget) -> Result<()>;
    fn unstage_file_in_repo(&self, repo: &RepoSelector, target: &ScPathTarget) -> Result<()>;
    fn list_changes_in_repo(&self, repo: &RepoSelector) -> Result<Vec<ChangeEntry>>;
    fn diff_doc_path_in_repo(&self, repo: &RepoSelector, target: &ScPathTarget) -> Result<String>;
    fn list_commits_in_repo(&self, repo: &RepoSelector, limit: u32) -> Result<Vec<CommitInfo>>;
    fn diff_commits_in_repo(
        &self,
        repo: &RepoSelector,
        commit_a_id: Option<&str>,
        commit_b_id: &str,
    ) -> Result<Vec<CommitFileDiff>>;
    fn commit_staged_in_repo(&self, repo: &RepoSelector, message: &str) -> Result<CommitInfo>;
}
