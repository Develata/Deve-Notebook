//! plan_ref:
//!   - 06_repository#repo-catalog-contract
//!   - 06_repository#repo-scope-runtime

use crate::ledger::manager::local_repo_metadata_repair::validate_local_repo_metadata;
use crate::ledger::manager::types::RepoManager;
use crate::models::PeerId;
use anyhow::Result;

impl RepoManager {
    pub(crate) fn refresh_local_repo_catalog(&self) -> Result<()> {
        validate_local_repo_metadata(
            &self.ledger_dir,
            &self.local_repo_name,
            self.local_db.as_ref(),
        )?;
        Self::validate_local_repo_source_control_tables(
            &self.ledger_dir,
            &self.local_repo_name,
            self.local_db.as_ref(),
        )
    }
}

pub(crate) struct RepoCatalogRuntime<'a> {
    pub(super) manager: &'a RepoManager,
}

impl<'a> RepoCatalogRuntime<'a> {
    pub(crate) fn new(manager: &'a RepoManager) -> Self {
        Self { manager }
    }

    pub(crate) fn refresh_local_catalog(&self) -> Result<()> {
        self.manager.refresh_local_repo_catalog()
    }

    pub(crate) fn list_repos(&self, peer_id: Option<&PeerId>) -> Result<Vec<String>> {
        if let Some(peer_id) = peer_id {
            return self.list_remote_repo_names(peer_id);
        }
        self.list_local_display_names()
    }
}

impl RepoManager {
    pub(crate) fn repo_catalog_runtime(&self) -> RepoCatalogRuntime<'_> {
        RepoCatalogRuntime::new(self)
    }
}
