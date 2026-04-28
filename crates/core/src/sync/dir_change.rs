//! plan_ref:
//!   - 04_storage#watcher-contract

use super::SyncManager;
use crate::models::RepoId;
use anyhow::Result;

impl SyncManager {
    pub fn handle_dir_change(&self, path_str: &str) -> Result<Option<(RepoId, String)>> {
        let Some((repo_name, repo_id, repo_path)) =
            self.repo.resolve_local_workspace_path(path_str)?
        else {
            return Ok(None);
        };
        if repo_path.is_empty() {
            return Ok(None);
        }
        if !self.dir_refresh_guard.should_refresh(repo_id) {
            return Ok(None);
        }
        super::scan::scan_local_repo(&self.repo, &self.vfs, &repo_name)?;
        Ok(Some((repo_id, repo_path)))
    }
}
