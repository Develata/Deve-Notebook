// crates/core/src/ledger/traits.rs
//! # Repository Trait

use crate::models::DocId;
use crate::source_control::{ChangeEntry, CommitInfo};
use anyhow::Result;

pub trait Repository: Send + Sync {
    fn list_docs(&self) -> Result<Vec<(DocId, String)>>;
    fn get_doc_content(&self, doc_id: DocId) -> Result<String>;

    fn list_changes(&self) -> Result<Vec<ChangeEntry>>;
    fn diff_doc_path(&self, path: &str) -> Result<String>;
    fn stage_file(&self, path: &str) -> Result<()>;
    fn commit_staged(&self, message: &str) -> Result<CommitInfo>;
}
