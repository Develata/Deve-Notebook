// crates/core/src/ledger/manager/repository.rs
//! # Repository Trait 实现 (RepoManager)

use crate::ledger::listing::RepoListing;
use crate::ledger::traits::Repository;
use crate::ledger::RepoManager;
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

    fn get_doc_content(&self, doc_id: DocId) -> Result<String> {
        let ops = self.get_local_ops(doc_id)?;
        let entries: Vec<_> = ops.into_iter().map(|(_, e)| e).collect();
        Ok(reconstruct_content(&entries))
    }

    fn list_changes(&self) -> Result<Vec<ChangeEntry>> {
        self.list_changes()
    }

    fn diff_doc_path(&self, path: &str) -> Result<String> {
        self.diff_doc_path(path)
    }

    fn stage_file(&self, path: &str) -> Result<()> {
        self.stage_file(path)
    }

    fn commit_staged(&self, message: &str) -> Result<CommitInfo> {
        self.commit_staged(message)
    }
}
