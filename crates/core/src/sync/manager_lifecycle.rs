//! plan_ref:
//!   - 06_repository#tree-projection-contract
//!   - 04_storage#projection-contract

use super::{DirRefreshGuard, ProjectionHealth, SyncManager, materialize, scan};
use crate::ledger::RepoManager;
use crate::vfs::Vfs;
use anyhow::Result;
use std::path::PathBuf;
use std::sync::Arc;

impl SyncManager {
    pub fn new(repo: Arc<RepoManager>, vault_root: PathBuf) -> Self {
        let vfs = Vfs::new(&vault_root);
        Self {
            dir_refresh_guard: DirRefreshGuard::new(),
            persist_guard: repo.persist_guard.clone(),
            repo,
            vault_root,
            vfs,
            projection_health: ProjectionHealth::new(),
        }
    }

    pub fn new_checked(repo: Arc<RepoManager>, vault_root: PathBuf) -> Result<Self> {
        let vfs = Vfs::new_checked(&vault_root)?;
        Ok(Self {
            dir_refresh_guard: DirRefreshGuard::new(),
            persist_guard: repo.persist_guard.clone(),
            repo,
            vault_root,
            vfs,
            projection_health: ProjectionHealth::new(),
        })
    }

    pub fn scan(&self) -> Result<()> {
        let degraded = materialize::prepare_local_workspaces(
            &self.repo,
            &self.vault_root,
            &self.persist_guard,
        )?;
        let degraded_set = degraded.iter().cloned().collect();
        self.replace_projection_degraded(&degraded);
        scan::scan_vault_excluding(&self.repo, &self.vfs, &self.vault_root, &degraded_set)
    }
}
