// crates/core/src/ledger/manager/source_control_api.rs
//! # Source Control API 实现 (RepoManager)

use crate::ledger::RepoManager;
use crate::ledger::traits::RepoSelector;
use crate::source_control::{ChangeEntry, CommitInfo, SourceControlApi};
use anyhow::Result;

impl SourceControlApi for RepoManager {
    fn list_pending_fs_in_repo(&self, repo: &RepoSelector) -> Result<Vec<ChangeEntry>> {
        let repo_name = self.resolve_local_repo_name(repo.repo_id, repo.repo_name.as_deref())?;
        self.list_pending_fs_in_local_repo(&repo_name)
    }

    fn stage_pending_in_repo(&self, repo: &RepoSelector, path: &str) -> Result<()> {
        let repo_name = self.resolve_local_repo_name(repo.repo_id, repo.repo_name.as_deref())?;
        self.stage_pending_in_local_repo(&repo_name, path)
    }

    fn discard_pending_in_repo(&self, repo: &RepoSelector, path: &str) -> Result<()> {
        let repo_name = self.resolve_local_repo_name(repo.repo_id, repo.repo_name.as_deref())?;
        self.discard_pending_in_local_repo(&repo_name, path)
    }

    fn list_changes_in_repo(&self, repo: &RepoSelector) -> Result<Vec<ChangeEntry>> {
        let repo_name = self.resolve_local_repo_name(repo.repo_id, repo.repo_name.as_deref())?;
        self.list_changes_in_local_repo(&repo_name)
    }

    fn diff_doc_path_in_repo(&self, repo: &RepoSelector, path: &str) -> Result<String> {
        let repo_name = self.resolve_local_repo_name(repo.repo_id, repo.repo_name.as_deref())?;
        self.diff_doc_path_in_local_repo(&repo_name, path)
    }

    fn stage_file_in_repo(&self, repo: &RepoSelector, path: &str) -> Result<()> {
        let repo_name = self.resolve_local_repo_name(repo.repo_id, repo.repo_name.as_deref())?;
        self.stage_file_in_local_repo(&repo_name, path)
    }

    fn commit_staged_in_repo(&self, repo: &RepoSelector, message: &str) -> Result<CommitInfo> {
        let repo_name = self.resolve_local_repo_name(repo.repo_id, repo.repo_name.as_deref())?;
        self.commit_staged_in_local_repo(&repo_name, message)
    }
}
