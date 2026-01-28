// crates/core/src/ledger/manager/source_control_api.rs
//! # Source Control API 实现 (RepoManager)

use crate::ledger::RepoManager;
use crate::source_control::{ChangeEntry, CommitInfo, SourceControlApi};
use anyhow::Result;

impl SourceControlApi for RepoManager {
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
