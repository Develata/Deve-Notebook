//! plan_ref:
//!   - 03_storage/watcher#watcher-contract

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
        if !self.dir_refresh_guard.should_refresh(repo_id) {
            return Ok(None);
        }
        self.force_dir_refresh(repo_name, repo_id, repo_path)
            .map(Some)
    }

    pub(crate) fn force_dir_refresh(
        &self,
        repo_name: &str,
        repo_id: RepoId,
        repo_path: &str,
    ) -> Result<(RepoId, String)> {
        super::scan::scan_local_repo(&self.repo, &self.vfs, repo_name)?;
        Ok((repo_id, repo_path.to_string()))
    }
}
