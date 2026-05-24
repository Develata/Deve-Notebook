//! plan_ref:
//!   - 06_repository#tree-projection-contract
//!   - 04_storage#projection-contract

use super::{DirRefreshGuard, ProjectionHealth, SyncManager, materialize, scan};
use crate::ledger::RepoManager;
use crate::vfs::Vfs;
use anyhow::Result;
use std::sync::Arc;

impl SyncManager {
    pub fn new_checked(repo: Arc<RepoManager>) -> Result<Self> {
        repo.list_local_repo_names_for_execution()?;
        repo.validate_projection_locator_map()?;
        Ok(Self {
            dir_refresh_guard: DirRefreshGuard::new(),
            persist_guard: repo.persist_guard.clone(),
            repo,
            vfs: Vfs::unrooted(),
            projection_health: ProjectionHealth::new(),
        })
    }

    pub fn scan(&self) -> Result<()> {
        let degraded = materialize::prepare_local_workspaces(&self.repo, &self.persist_guard)?;
        let degraded_set = degraded.iter().cloned().collect();
        self.replace_projection_degraded(&degraded);
        scan::scan_all_local_repos_excluding(&self.repo, &self.vfs, &degraded_set)
    }
}
