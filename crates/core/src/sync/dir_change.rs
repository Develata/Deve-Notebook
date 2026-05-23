//! plan_ref:
//!   - 04_storage#watcher-contract

use super::SyncManager;
use crate::models::RepoId;
use anyhow::Result;

impl SyncManager {
    pub fn handle_dir_change(
        &self,
        repo_name: &str,
        repo_id: RepoId,
        repo_path: &str,
    ) -> Result<Option<(RepoId, String)>> {
        if repo_path.is_empty() {
            return Ok(None);
        }
        if !self.dir_refresh_guard.should_refresh(repo_id) {
            return Ok(None);
        }
        super::scan::scan_local_repo(&self.repo, &self.vfs, &repo_name)?;
        Ok(Some((repo_id, repo_path.to_string())))
    }
}
