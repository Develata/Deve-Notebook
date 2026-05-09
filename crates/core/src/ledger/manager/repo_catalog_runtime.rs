//! plan_ref:
//!   - 06_repository#repo-catalog-contract
//!   - 06_repository#repo-scope-runtime

use crate::ledger::listing::RepoListing;
use crate::ledger::manager::types::RepoManager;
use anyhow::Result;

impl RepoManager {
    pub(crate) fn refresh_local_repo_catalog(&self) -> Result<()> {
        Self::validate_local_repo_metadata(
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
    manager: &'a RepoManager,
}

impl<'a> RepoCatalogRuntime<'a> {
    pub(crate) fn new(manager: &'a RepoManager) -> Self {
        Self { manager }
    }

    pub(crate) fn refresh_local_catalog(&self) -> Result<()> {
        self.manager.refresh_local_repo_catalog()
    }

    pub(crate) fn list_local_display_names(&self) -> Result<Vec<String>> {
        self.manager.list_repos(None)
    }

    pub(crate) fn list_local_execution_names(&self) -> Result<Vec<String>> {
        self.manager.list_local_repo_names_for_execution()
    }
}

impl RepoManager {
    pub(crate) fn repo_catalog_runtime(&self) -> RepoCatalogRuntime<'_> {
        RepoCatalogRuntime::new(self)
    }
}
