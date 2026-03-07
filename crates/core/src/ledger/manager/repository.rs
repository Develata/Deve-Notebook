// crates/core/src/ledger/manager/repository.rs
//! # Repository Trait 实现 (RepoManager)

use crate::ledger::RepoManager;
use crate::ledger::listing::RepoListing;
use crate::ledger::traits::{RepoSelector, Repository};
use crate::models::DocId;
use crate::models::RepoType;
use crate::source_control::{ChangeEntry, CommitInfo};
use crate::state::reconstruct_content;
use anyhow::Result;

impl Repository for RepoManager {
    fn list_docs(&self) -> Result<Vec<(DocId, String)>> {
        let repo_id = self
            .get_repo_info()
            .ok()
            .flatten()
            .map(|info| info.uuid)
            .unwrap_or_else(uuid::Uuid::nil);
        RepoListing::list_docs(self, &RepoType::Local(repo_id))
    }

    fn list_docs_in_repo(&self, repo: &RepoSelector) -> Result<Vec<(DocId, String)>> {
        let repo_name = self.resolve_local_repo_name(repo.repo_id, repo.repo_name.as_deref())?;
        self.list_local_docs(Some(&repo_name))
    }

    fn get_doc_content(&self, doc_id: DocId) -> Result<String> {
        let ops = self.get_local_ops(doc_id)?;
        let entries: Vec<_> = ops.into_iter().map(|(_, e)| e).collect();
        Ok(reconstruct_content(&entries))
    }

    fn get_doc_content_in_repo(&self, repo: &RepoSelector, doc_id: DocId) -> Result<String> {
        let repo_name = self.resolve_local_repo_name(repo.repo_id, repo.repo_name.as_deref())?;
        let ops = self.get_local_ops_in_local_repo(&repo_name, doc_id)?;
        let entries: Vec<_> = ops.into_iter().map(|(_, e)| e).collect();
        Ok(reconstruct_content(&entries))
    }

    fn list_pending_fs(&self) -> Result<Vec<ChangeEntry>> {
        self.list_pending_fs()
    }

    fn stage_pending(&self, path: &str) -> Result<()> {
        self.stage_pending(path)
    }

    fn discard_pending(&self, path: &str) -> Result<()> {
        self.discard_pending(path)
    }

    fn list_changes(&self) -> Result<Vec<ChangeEntry>> {
        self.list_changes()
    }

    fn list_changes_in_repo(&self, repo: &RepoSelector) -> Result<Vec<ChangeEntry>> {
        let repo_name = self.resolve_local_repo_name(repo.repo_id, repo.repo_name.as_deref())?;
        self.list_changes_in_local_repo(&repo_name)
    }

    fn diff_doc_path(&self, path: &str) -> Result<String> {
        self.diff_doc_path(path)
    }

    fn diff_doc_path_in_repo(&self, repo: &RepoSelector, path: &str) -> Result<String> {
        let repo_name = self.resolve_local_repo_name(repo.repo_id, repo.repo_name.as_deref())?;
        self.diff_doc_path_in_local_repo(&repo_name, path)
    }

    fn stage_file(&self, path: &str) -> Result<()> {
        self.stage_file(path)
    }

    fn stage_file_in_repo(&self, repo: &RepoSelector, path: &str) -> Result<()> {
        let repo_name = self.resolve_local_repo_name(repo.repo_id, repo.repo_name.as_deref())?;
        self.stage_file_in_local_repo(&repo_name, path)
    }

    fn commit_staged(&self, message: &str) -> Result<CommitInfo> {
        self.commit_staged(message)
    }

    fn commit_staged_in_repo(&self, repo: &RepoSelector, message: &str) -> Result<CommitInfo> {
        let repo_name = self.resolve_local_repo_name(repo.repo_id, repo.repo_name.as_deref())?;
        self.commit_staged_in_local_repo(&repo_name, message)
    }
}
