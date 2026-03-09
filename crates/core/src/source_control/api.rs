// crates/core/src/source_control/api.rs
//! # Source Control API (Trait)

use crate::ledger::traits::RepoSelector;
use crate::source_control::{ChangeEntry, CommitInfo};
use anyhow::Result;

pub trait SourceControlApi: Send + Sync {
    fn list_pending_fs_in_repo(&self, repo: &RepoSelector) -> Result<Vec<ChangeEntry>>;
    fn stage_pending_in_repo(&self, repo: &RepoSelector, path: &str) -> Result<()>;
    fn discard_pending_in_repo(&self, repo: &RepoSelector, path: &str) -> Result<()>;
    fn list_changes_in_repo(&self, repo: &RepoSelector) -> Result<Vec<ChangeEntry>>;
    fn diff_doc_path_in_repo(&self, repo: &RepoSelector, path: &str) -> Result<String>;
    fn stage_file_in_repo(&self, repo: &RepoSelector, path: &str) -> Result<()>;
    fn commit_staged_in_repo(&self, repo: &RepoSelector, message: &str) -> Result<CommitInfo>;
}
