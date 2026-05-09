// crates/core/src/ledger/traits.rs
//! plan_ref:
//!   - 06_repository#repo-selector-resolution-contract
//!   - 06_repository#repo-scope-runtime
//!   - 07_diff_logic#source-control-runtime
//!
//! # Repository Trait

use crate::models::{DocId, RepoId};
use crate::source_control::SourceControlApi;
use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RepoSelector {
    #[serde(default)]
    pub repo_id: Option<RepoId>,
    #[serde(default)]
    pub repo_name: Option<String>,
}

pub trait Repository: SourceControlApi {
    fn list_docs_in_repo(&self, repo: &RepoSelector) -> Result<Vec<(DocId, String)>>;
    fn get_doc_content_in_repo(&self, repo: &RepoSelector, doc_id: DocId) -> Result<String>;
}
