//! plan_ref:
//!   - 04_repository#repo-catalog-contract
//!   - 04_repository#repo-scope-runtime
//!   - 04_repository#repo-lifecycle-coordinator

mod cut;
mod error;
mod membership;
mod model;
mod prepared_identity;
mod store;

pub use error::RepoCatalogError;
pub use membership::{
    CatalogMembershipError, CatalogMembershipGeneration, CatalogMembershipRuntime,
    CatalogMembershipToken, RepoCatalogCutAuthority, RepoCatalogCutPermit,
};
pub use model::{
    PreparedRepoCreation, PreparedRepoIdentity, PreparedRepoRemoval, RepoCatalogCreationCommit,
    RepoCatalogMembershipRecord, RepoCatalogMembershipState, RepoCatalogRemovalCommit,
    RevalidatedRepoCreation, RevalidatedRepoRemoval,
};

use crate::ledger::manager::local_repo_metadata_repair::validate_local_repo_metadata;
use crate::ledger::manager::types::RepoManager;
use crate::models::PeerId;
use crate::models::RepoId;
use anyhow::Result;
use std::path::Path;

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

/// Lists the RepoIds with a durable `Normal` catalog membership record,
/// without constructing a `RepoManager`. Hosts use this to decide bootstrap
/// ("zero live repos") before any repo database is created or opened.
pub fn normal_catalog_ids_for_ledger(ledger_dir: &Path) -> Result<Vec<RepoId>, RepoCatalogError> {
    let store = store::RepoCatalogStore::open(ledger_dir)?;
    let mut ids = store
        .list()?
        .into_iter()
        .filter_map(|record| {
            (record.state() == RepoCatalogMembershipState::Normal).then_some(record.repo_id())
        })
        .collect::<Vec<_>>();
    ids.sort_unstable();
    Ok(ids)
}

#[cfg(test)]
mod tests;

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

    /// Returns the process-local catalog membership authority capability.
    ///
    /// Tokens issued by this runtime are readiness evidence only. They do not
    /// mutate durable catalog state and cannot outlive or impersonate another
    /// `RepoManager` composition runtime.
    pub fn catalog_membership_runtime(&self) -> CatalogMembershipRuntime {
        self.catalog_membership.clone()
    }

    /// Claims the unique host mutation capability for the catalog cut lane.
    /// A second concurrent composition root for the same ledger fails closed.
    pub fn claim_repo_catalog_cut_authority(
        &self,
    ) -> Result<RepoCatalogCutAuthority, CatalogMembershipError> {
        self.catalog_membership.claim_cut_authority()
    }
}
