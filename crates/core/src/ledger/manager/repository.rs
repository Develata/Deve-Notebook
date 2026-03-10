// crates/core/src/ledger/manager/repository.rs
//! # Repository Trait 实现 (RepoManager)

use crate::ledger::RepoManager;
use crate::ledger::traits::{RepoSelector, Repository};
use crate::models::DocId;
use crate::protocol::ScPathTarget;
use crate::source_control::{ChangeEntry, CommitFileDiff, CommitInfo};
use crate::state::reconstruct_content;
use anyhow::Result;

impl Repository for RepoManager {
    fn list_docs_in_repo(&self, repo: &RepoSelector) -> Result<Vec<(DocId, String)>> {
        let repo_name = self.resolve_local_repo_name(repo.repo_id, repo.repo_name.as_deref())?;
        self.list_local_docs(Some(&repo_name))
    }

    fn get_doc_content_in_repo(&self, repo: &RepoSelector, doc_id: DocId) -> Result<String> {
        let repo_name = self.resolve_local_repo_name(repo.repo_id, repo.repo_name.as_deref())?;
        let ops = self.get_local_ops_in_local_repo(&repo_name, doc_id)?;
        let entries: Vec<_> = ops.into_iter().map(|(_, e)| e).collect();
        Ok(reconstruct_content(&entries))
    }

    fn list_pending_fs_in_repo(&self, repo: &RepoSelector) -> Result<Vec<ChangeEntry>> {
        let repo_name = self.resolve_local_repo_name(repo.repo_id, repo.repo_name.as_deref())?;
        self.list_pending_fs_in_local_repo(&repo_name)
    }

    fn stage_pending_in_repo(&self, repo: &RepoSelector, target: &ScPathTarget) -> Result<()> {
        let repo_name = self.resolve_local_repo_name(repo.repo_id, repo.repo_name.as_deref())?;
        self.stage_pending_target_in_local_repo(&repo_name, target)
    }

    fn discard_pending_in_repo(&self, repo: &RepoSelector, target: &ScPathTarget) -> Result<()> {
        let repo_name = self.resolve_local_repo_name(repo.repo_id, repo.repo_name.as_deref())?;
        self.discard_pending_target_in_local_repo(&repo_name, target)
    }

    fn unstage_file_in_repo(&self, repo: &RepoSelector, target: &ScPathTarget) -> Result<()> {
        let repo_name = self.resolve_local_repo_name(repo.repo_id, repo.repo_name.as_deref())?;
        self.unstage_file_target_in_local_repo(&repo_name, target)
    }

    fn list_changes_in_repo(&self, repo: &RepoSelector) -> Result<Vec<ChangeEntry>> {
        let repo_name = self.resolve_local_repo_name(repo.repo_id, repo.repo_name.as_deref())?;
        self.list_changes_in_local_repo(&repo_name)
    }

    fn diff_doc_path_in_repo(&self, repo: &RepoSelector, target: &ScPathTarget) -> Result<String> {
        let repo_name = self.resolve_local_repo_name(repo.repo_id, repo.repo_name.as_deref())?;
        self.diff_doc_target_in_local_repo(&repo_name, target)
    }

    fn list_commits_in_repo(&self, repo: &RepoSelector, limit: u32) -> Result<Vec<CommitInfo>> {
        let repo_name = self.resolve_local_repo_name(repo.repo_id, repo.repo_name.as_deref())?;
        self.list_commits_in_local_repo(&repo_name, limit)
    }

    fn diff_commits_in_repo(
        &self,
        repo: &RepoSelector,
        commit_a_id: Option<&str>,
        commit_b_id: &str,
    ) -> Result<Vec<CommitFileDiff>> {
        let repo_name = self.resolve_local_repo_name(repo.repo_id, repo.repo_name.as_deref())?;
        self.diff_commits_in_local_repo(&repo_name, commit_a_id, commit_b_id)
    }

    fn commit_staged_in_repo(&self, repo: &RepoSelector, message: &str) -> Result<CommitInfo> {
        let repo_name = self.resolve_local_repo_name(repo.repo_id, repo.repo_name.as_deref())?;
        self.commit_staged_in_local_repo(&repo_name, message)
    }
}
