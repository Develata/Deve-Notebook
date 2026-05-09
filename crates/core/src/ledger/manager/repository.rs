// crates/core/src/ledger/manager/repository.rs
//! plan_ref:
//!   - 06_repository#repo-selector-resolution-contract
//!   - 06_repository#repo-scope-runtime
//!   - 07_diff_logic#source-control-runtime
//!
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
        let repo_name = self
            .repo_scope_runtime()
            .resolve_local_selector_for_execution(repo)?;
        self.list_local_docs(Some(&repo_name))
    }

    fn get_doc_content_in_repo(&self, repo: &RepoSelector, doc_id: DocId) -> Result<String> {
        let repo_name = self
            .repo_scope_runtime()
            .resolve_local_selector_for_execution(repo)?;
        if self
            .get_file_meta_for_doc_in_local_repo(&repo_name, doc_id)?
            .is_none()
        {
            anyhow::bail!("Document not found: {}", doc_id);
        }
        let ops = self.get_local_ops_in_local_repo(&repo_name, doc_id)?;
        let entries: Vec<_> = ops.into_iter().map(|(_, e)| e).collect();
        Ok(reconstruct_content(&entries))
    }

    fn list_pending_fs_in_repo(&self, repo: &RepoSelector) -> Result<Vec<ChangeEntry>> {
        self.source_control_runtime().list_pending_fs_in_repo(repo)
    }

    fn stage_pending_in_repo(&self, repo: &RepoSelector, target: &ScPathTarget) -> Result<()> {
        self.source_control_runtime()
            .stage_pending_in_repo(repo, target)
    }

    fn discard_pending_in_repo(&self, repo: &RepoSelector, target: &ScPathTarget) -> Result<()> {
        self.source_control_runtime()
            .discard_pending_in_repo(repo, target)
    }

    fn unstage_file_in_repo(&self, repo: &RepoSelector, target: &ScPathTarget) -> Result<()> {
        self.source_control_runtime()
            .unstage_file_in_repo(repo, target)
    }

    fn list_changes_in_repo(&self, repo: &RepoSelector) -> Result<Vec<ChangeEntry>> {
        self.source_control_runtime().list_changes_in_repo(repo)
    }

    fn diff_doc_path_in_repo(&self, repo: &RepoSelector, target: &ScPathTarget) -> Result<String> {
        self.source_control_runtime()
            .diff_doc_path_in_repo(repo, target)
    }

    fn list_commits_in_repo(&self, repo: &RepoSelector, limit: u32) -> Result<Vec<CommitInfo>> {
        self.source_control_runtime()
            .list_commits_in_repo(repo, limit)
    }

    fn diff_commits_in_repo(
        &self,
        repo: &RepoSelector,
        commit_a_id: Option<&str>,
        commit_b_id: &str,
    ) -> Result<Vec<CommitFileDiff>> {
        self.source_control_runtime()
            .diff_commits_in_repo(repo, commit_a_id, commit_b_id)
    }

    fn commit_staged_in_repo(&self, repo: &RepoSelector, message: &str) -> Result<CommitInfo> {
        self.source_control_runtime()
            .commit_staged_in_repo(repo, message)
    }
}
