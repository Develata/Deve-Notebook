// crates/core/src/ledger/traits.rs
//! # Repository Trait

use crate::models::{DocId, RepoId};
use crate::source_control::{ChangeEntry, CommitFileDiff, CommitInfo};
use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RepoSelector {
    #[serde(default)]
    pub repo_id: Option<RepoId>,
    #[serde(default)]
    pub repo_name: Option<String>,
}

pub trait Repository: Send + Sync {
    fn list_docs_in_repo(&self, repo: &RepoSelector) -> Result<Vec<(DocId, String)>>;
    fn get_doc_content_in_repo(&self, repo: &RepoSelector, doc_id: DocId) -> Result<String>;
    fn list_pending_fs_in_repo(&self, repo: &RepoSelector) -> Result<Vec<ChangeEntry>>;
    fn stage_pending_in_repo(&self, repo: &RepoSelector, path: &str) -> Result<()>;
    fn discard_pending_in_repo(&self, repo: &RepoSelector, path: &str) -> Result<()>;
    fn unstage_file_in_repo(&self, repo: &RepoSelector, path: &str) -> Result<()>;
    fn list_changes_in_repo(&self, repo: &RepoSelector) -> Result<Vec<ChangeEntry>>;
    fn diff_doc_path_in_repo(&self, repo: &RepoSelector, path: &str) -> Result<String>;
    fn stage_file_in_repo(&self, repo: &RepoSelector, path: &str) -> Result<()>;
    fn list_commits_in_repo(&self, repo: &RepoSelector, limit: u32) -> Result<Vec<CommitInfo>>;
    fn diff_commits_in_repo(
        &self,
        repo: &RepoSelector,
        commit_a_id: Option<&str>,
        commit_b_id: &str,
    ) -> Result<Vec<CommitFileDiff>>;
    fn commit_staged_in_repo(&self, repo: &RepoSelector, message: &str) -> Result<CommitInfo>;
}
