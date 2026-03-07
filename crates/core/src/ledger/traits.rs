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
    fn list_docs(&self) -> Result<Vec<(DocId, String)>>;
    fn list_docs_in_repo(&self, repo: &RepoSelector) -> Result<Vec<(DocId, String)>> {
        let _ = repo;
        self.list_docs()
    }

    fn get_doc_content(&self, doc_id: DocId) -> Result<String>;
    fn get_doc_content_in_repo(&self, repo: &RepoSelector, doc_id: DocId) -> Result<String> {
        let _ = repo;
        self.get_doc_content(doc_id)
    }

    // --- Pending (Working Directory) ---
    fn list_pending_fs(&self) -> Result<Vec<ChangeEntry>>;
    fn stage_pending(&self, path: &str) -> Result<()>;
    fn discard_pending(&self, path: &str) -> Result<()>;

    // --- Staging & Commit ---
    fn list_changes(&self) -> Result<Vec<ChangeEntry>>;
    fn list_changes_in_repo(&self, repo: &RepoSelector) -> Result<Vec<ChangeEntry>> {
        let _ = repo;
        self.list_changes()
    }

    fn diff_doc_path(&self, path: &str) -> Result<String>;
    fn diff_doc_path_in_repo(&self, repo: &RepoSelector, path: &str) -> Result<String> {
        let _ = repo;
        self.diff_doc_path(path)
    }

    fn stage_file(&self, path: &str) -> Result<()>;
    fn stage_file_in_repo(&self, repo: &RepoSelector, path: &str) -> Result<()> {
        let _ = repo;
        self.stage_file(path)
    }

    fn commit_staged(&self, message: &str) -> Result<CommitInfo>;
    fn commit_staged_in_repo(&self, repo: &RepoSelector, message: &str) -> Result<CommitInfo> {
        let _ = repo;
        self.commit_staged(message)
    }
}
