//! plan_ref:
//!   - 04_repository#tree-projection-contract
//!   - 03_storage/projection#projection-contract

use super::{
    DirRefreshGuard, ProjectionHealth, SyncManager, materialize, projection_fault_journal, scan,
};
use crate::ledger::RepoManager;
use crate::vfs::Vfs;
use anyhow::Result;
use std::collections::HashSet;
use std::sync::Arc;

impl SyncManager {
    pub fn new_checked(repo: Arc<RepoManager>) -> Result<Self> {
        repo.list_local_repo_names_for_execution()?;
        repo.validate_projection_locator_map()?;
        let manager = Self {
            dir_refresh_guard: DirRefreshGuard::new(),
            persist_guard: repo.persist_guard.clone(),
            repo,
            vfs: Vfs::unrooted(),
            projection_health: ProjectionHealth::new(),
        };
        manager.load_durable_projection_faults()?;
        Ok(manager)
    }

    pub fn scan(&self) -> Result<()> {
        let mut degraded = projection_fault_journal::load_degraded_repo_names(&self.repo)?;
        let durable_degraded: HashSet<String> = degraded.iter().cloned().collect();
        degraded.extend(materialize::prepare_local_workspaces(
            &self.repo,
            &self.persist_guard,
            &durable_degraded,
        )?);
        degraded.sort();
        degraded.dedup();
        let degraded_set = degraded.iter().cloned().collect();
        self.replace_projection_degraded(&degraded);
        scan::scan_all_local_repos_excluding(&self.repo, &self.vfs, &degraded_set)
    }

    pub fn scan_repo(&self, repo_name: &str) -> Result<()> {
        let degraded = projection_fault_journal::load_degraded_repo_names(&self.repo)?;
        if degraded.iter().any(|name| name == repo_name) {
            anyhow::bail!("Projection workspace for repo {repo_name} is degraded; scan aborted");
        }
        scan::scan_local_repo(&self.repo, &self.vfs, repo_name)
    }

    fn load_durable_projection_faults(&self) -> Result<()> {
        let degraded = projection_fault_journal::load_degraded_repo_names(&self.repo)?;
        for repo_name in degraded {
            self.mark_projection_degraded(&repo_name);
        }
        Ok(())
    }
}
