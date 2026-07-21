//! plan_ref:
//!   - 04_repository#repo-catalog-contract
//!   - 04_repository#repo-scope-runtime
//!   - 04_repository#repo-lifecycle-coordinator

mod cut;
mod cut_failure;
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
        let normal_repo_ids = self.normal_repo_catalog_ids()?;
        validate_local_repo_metadata(
            self.local_repo_name(),
            &self.local_authority,
            &normal_repo_ids,
        )?;
        self.validate_local_repo_source_control_tables()
    }
}

/// Lists the RepoIds with a durable `Normal` catalog membership record,
/// without constructing a `RepoManager`. Hosts use this to decide bootstrap
/// ("zero live repos") before any repo database is created or opened.
pub fn normal_catalog_ids_for_ledger(ledger_dir: &Path) -> Result<Vec<RepoId>, RepoCatalogError> {
    Ok(catalog_bootstrap_snapshot_for_ledger(ledger_dir)?.normal_repo_ids)
}

/// Durable catalog truth used before a `RepoManager` composition root exists.
///
/// `has_records` deliberately distinguishes a never-cataloged bootstrap from
/// a legitimate zero-live-repo host whose durable records are all Removed.
/// Normal startup may inspect physical databases only in the first case.
#[derive(Debug)]
pub(crate) struct RepoCatalogBootstrapSnapshot {
    has_records: bool,
    normal_records: Vec<RepoCatalogMembershipRecord>,
    normal_repo_ids: Vec<RepoId>,
}

impl RepoCatalogBootstrapSnapshot {
    pub(crate) fn has_records(&self) -> bool {
        self.has_records
    }

    pub(crate) fn normal_repo_ids(&self) -> &[RepoId] {
        &self.normal_repo_ids
    }

    pub(crate) fn normal_record(&self, repo_id: RepoId) -> Option<&RepoCatalogMembershipRecord> {
        self.normal_records
            .iter()
            .find(|record| record.repo_id() == repo_id)
    }

    pub(crate) fn normal_records(&self) -> &[RepoCatalogMembershipRecord] {
        &self.normal_records
    }
}

pub(crate) fn catalog_bootstrap_snapshot_for_ledger(
    ledger_dir: &Path,
) -> Result<RepoCatalogBootstrapSnapshot, RepoCatalogError> {
    let store = store::RepoCatalogStore::open(ledger_dir)?;
    // This pre-composition probe has no process-local membership runtime yet.
    // The cross-process store lock is nevertheless mandatory because list()
    // removes project-owned crash temps and must not race a live publisher.
    let _store_lock = store.lock()?;
    let records = store.list()?;
    let has_records = !records.is_empty();
    let mut normal_records = records
        .into_iter()
        .filter(|record| record.state() == RepoCatalogMembershipState::Normal)
        .collect::<Vec<_>>();
    normal_records.sort_by_key(RepoCatalogMembershipRecord::repo_id);
    let normal_repo_ids = normal_records
        .iter()
        .map(RepoCatalogMembershipRecord::repo_id)
        .collect();
    Ok(RepoCatalogBootstrapSnapshot {
        has_records,
        normal_records,
        normal_repo_ids,
    })
}

pub(crate) fn prepared_identity_for_existing_database(
    ledger_dir: &Path,
    repo_id: RepoId,
    db: &redb::Database,
) -> Result<PreparedRepoIdentity, RepoCatalogError> {
    prepared_identity::snapshot_from_db_at(ledger_dir, repo_id, db).map_err(|error| {
        RepoCatalogError::PreparedIdentityUnavailable {
            repo_id,
            detail: error.to_string(),
        }
    })
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
