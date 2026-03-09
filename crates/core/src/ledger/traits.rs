// crates/core/src/ledger/traits.rs
//! # Repository Trait

use crate::models::{DocId, RepoId};
use crate::source_control::{ChangeEntry, CommitInfo};
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
    fn list_changes_in_repo(&self, repo: &RepoSelector) -> Result<Vec<ChangeEntry>>;
    fn diff_doc_path_in_repo(&self, repo: &RepoSelector, path: &str) -> Result<String>;
    fn stage_file_in_repo(&self, repo: &RepoSelector, path: &str) -> Result<()>;
    fn commit_staged_in_repo(&self, repo: &RepoSelector, message: &str) -> Result<CommitInfo>;
}
