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
}
