// crates/core/src/source_control/api.rs
//! # Source Control API (Trait)

use crate::source_control::{ChangeEntry, CommitInfo};
use anyhow::Result;

pub trait SourceControlApi: Send + Sync {
    fn list_changes(&self) -> Result<Vec<ChangeEntry>>;
    fn diff_doc_path(&self, path: &str) -> Result<String>;
    fn stage_file(&self, path: &str) -> Result<()>;
    fn commit_staged(&self, message: &str) -> Result<CommitInfo>;
}
