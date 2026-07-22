//! plan_ref:
//!   - 04_repository#repo-catalog-contract
//!   - 04_repository#repo-lifecycle-coordinator
//!
//! RepoManager facade for prepared catalog membership cuts.

use super::*;
use crate::ledger::PreparedRepoAuthority;
use crate::ledger::manager::types::RepoManager;

impl RepoManager {
    pub fn prepare_repo_creation_membership(
        &self,
        repo_id: RepoId,
        lifecycle_request_id: Uuid,
    ) -> Result<PreparedRepoCreation, RepoCatalogError> {
        self.repo_catalog_runtime()
            .prepare_creation(repo_id, lifecycle_request_id)
    }

    pub fn revalidate_repo_creation_membership(
        &self,
        prepared: &PreparedRepoCreation,
    ) -> Result<RevalidatedRepoCreation, RepoCatalogError> {
        self.repo_catalog_runtime().revalidate_creation(prepared)
    }

    pub fn prepare_repo_creation_membership_with_authority(
        &self,
        repo_id: RepoId,
        lifecycle_request_id: Uuid,
        authority: &PreparedRepoAuthority,
    ) -> Result<PreparedRepoCreation, RepoCatalogError> {
        let identity = prepared_identity::snapshot_prepared(self, repo_id, authority)?;
        self.repo_catalog_runtime().prepare_creation_with_identity(
            repo_id,
            lifecycle_request_id,
            identity,
        )
    }

    pub fn revalidate_repo_creation_membership_with_authority(
        &self,
        prepared: &PreparedRepoCreation,
        authority: &PreparedRepoAuthority,
    ) -> Result<RevalidatedRepoCreation, RepoCatalogError> {
        let observed = prepared_identity::snapshot_prepared(self, prepared.repo_id, authority)?;
        self.repo_catalog_runtime()
            .revalidate_creation_with_identity(prepared, observed)
    }

    /// Commits only the bounded repo-catalog sub-cut. B1 exposes no public
    /// constructor for `RepoCatalogCutPermit`; C1' transfers its sole owner
    /// into the ordered publication gate.
    pub fn commit_repo_creation_membership(
        &self,
        prepared: &PreparedRepoCreation,
        revalidated: &RevalidatedRepoCreation,
        permit: &RepoCatalogCutPermit,
    ) -> Result<RepoCatalogCreationCommit, RepoCatalogError> {
        self.repo_catalog_runtime()
            .commit_creation(prepared, revalidated, permit)
    }

    pub fn prepare_repo_removal_membership(
        &self,
        repo_id: RepoId,
        lifecycle_request_id: Uuid,
        manifest_digest: &str,
    ) -> Result<PreparedRepoRemoval, RepoCatalogError> {
        self.repo_catalog_runtime()
            .prepare_removal(repo_id, lifecycle_request_id, manifest_digest)
    }

    pub fn revalidate_repo_removal_membership(
        &self,
        prepared: &PreparedRepoRemoval,
    ) -> Result<RevalidatedRepoRemoval, RepoCatalogError> {
        self.repo_catalog_runtime().revalidate_removal(prepared)
    }

    /// Commits only the bounded repo-catalog sub-cut. The complete immutable
    /// removal plan, including initiator and fallback outcome, belongs to the
    /// host lifecycle coordinator in C1'.
    pub fn commit_repo_removal_membership(
        &self,
        prepared: &PreparedRepoRemoval,
        revalidated: &RevalidatedRepoRemoval,
        permit: &RepoCatalogCutPermit,
    ) -> Result<RepoCatalogRemovalCommit, RepoCatalogError> {
        self.repo_catalog_runtime()
            .commit_removal(prepared, revalidated, permit)
    }

    pub fn repo_catalog_membership_record(
        &self,
        repo_id: RepoId,
    ) -> Result<Option<RepoCatalogMembershipRecord>, RepoCatalogError> {
        self.repo_catalog_runtime().record(repo_id)
    }

    pub fn retire_repo_removal_tombstone(
        &self,
        expected: &RepoCatalogMembershipRecord,
        permit: &RepoCatalogCutPermit,
    ) -> Result<RepoCatalogRetirementDisposition, RepoCatalogError> {
        self.repo_catalog_runtime().retire_removal(expected, permit)
    }

    /// Seeds process readiness exclusively from durable catalog records.
    pub fn seed_catalog_membership_from_records(&self) -> Result<(), RepoCatalogError> {
        self.repo_catalog_runtime().seed_process_membership()
    }

    pub fn normal_repo_catalog_ids(&self) -> Result<Vec<RepoId>, RepoCatalogError> {
        self.repo_catalog_runtime().normal_repo_ids()
    }
}
