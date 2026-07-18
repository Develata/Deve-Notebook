//! plan_ref:
//!   - 03_storage/watcher#watcher-contract

use super::SyncManager;
use crate::models::RepoId;
use anyhow::Result;
use std::path::Path;

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

    pub(crate) fn force_dir_refresh_at_root(
        &self,
        repo_name: &str,
        repo_id: RepoId,
        repo_root: &Path,
        repo_path: &str,
    ) -> Result<(RepoId, String)> {
        self.revalidate_watcher_repo_identity(repo_name, repo_id)?;
        super::scan::scan_local_repo_at_root(&self.repo, &self.vfs, repo_name, repo_root)?;
        self.revalidate_watcher_repo_identity(repo_name, repo_id)?;
        Ok((repo_id, repo_path.to_string()))
    }

    fn revalidate_watcher_repo_identity(&self, repo_name: &str, repo_id: RepoId) -> Result<()> {
        let info = self
            .repo
            .get_repo_info_for(None, Some(repo_name))?
            .ok_or_else(|| anyhow::anyhow!("watcher repository missing: {repo_name}"))?;
        anyhow::ensure!(
            info.uuid == repo_id,
            "watcher repository identity changed for {repo_name}: expected {repo_id}, found {}",
            info.uuid
        );
        Ok(())
    }
}
