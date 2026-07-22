//! plan_ref:
//!   - 04_repository#repo-catalog-contract
//!   - 04_repository#repo-lifecycle-coordinator
//!   - 03_storage/authority#repo-mutation-publication-gate

use super::cut_failure::{publish_error, publish_error_with_abort};
use super::error::state_name;
use super::model::{
    PreparedRepoCreation, PreparedRepoRemoval, RepoCatalogCreationCommit,
    RepoCatalogMembershipRecord, RepoCatalogMembershipState, RepoCatalogRemovalCommit,
    RepoCatalogRetirementDisposition, RevalidatedRepoCreation, RevalidatedRepoRemoval,
};
use super::prepared_identity;
use super::store::{RepoCatalogPublishPhase, RepoCatalogStore};
use super::{CatalogMembershipError, RepoCatalogCutPermit, RepoCatalogError, RepoCatalogRuntime};
use crate::models::RepoId;
use uuid::Uuid;

mod public_api;

impl RepoCatalogRuntime<'_> {
    pub(super) fn with_creation_activation_guard<T>(
        &self,
        creation: &PreparedRepoCreation,
        commit: &RepoCatalogCreationCommit,
        observed: super::PreparedRepoIdentity,
        activate: impl FnOnce() -> Result<T, crate::ledger::LocalAuthorityError>,
    ) -> Result<T, super::RepoCreationActivationError> {
        let repo_id = creation.repo_id;
        if observed != creation.prepared_identity
            || commit.record().repo_id() != repo_id
            || !commit
                .record()
                .confirms_created(creation.lifecycle_request_id)
            || commit.record().prepared_identity_digest() != creation.prepared_identity.to_hex()
        {
            return Err(super::RepoCreationActivationError::Catalog(
                RepoCatalogError::PreparedIdentityChanged(repo_id),
            ));
        }
        let _cut = self
            .manager
            .catalog_membership
            .cut_guard()
            .map_err(RepoCatalogError::from)
            .map_err(super::RepoCreationActivationError::Catalog)?;
        let store = RepoCatalogStore::open(&self.manager.ledger_dir)
            .map_err(super::RepoCreationActivationError::Catalog)?;
        let _process_lock = store
            .lock()
            .map_err(super::RepoCreationActivationError::Catalog)?;
        let current = store
            .load(repo_id)
            .map_err(super::RepoCreationActivationError::Catalog)?;
        if current.as_ref() != Some(commit.record()) {
            return Err(super::RepoCreationActivationError::Catalog(
                RepoCatalogError::PreparedIdentityChanged(repo_id),
            ));
        }
        self.manager
            .catalog_membership
            .with_activation_guard(commit.membership(), activate)
            .map_err(RepoCatalogError::from)
            .map_err(super::RepoCreationActivationError::Catalog)?
            .map_err(super::RepoCreationActivationError::Authority)
    }

    fn prepare_creation(
        &self,
        repo_id: RepoId,
        lifecycle_request_id: Uuid,
    ) -> Result<PreparedRepoCreation, RepoCatalogError> {
        let identity = prepared_identity::snapshot_initial_primary(self.manager, repo_id)?;
        self.prepare_creation_with_identity(repo_id, lifecycle_request_id, identity)
    }

    fn prepare_creation_with_identity(
        &self,
        repo_id: RepoId,
        lifecycle_request_id: Uuid,
        identity: super::PreparedRepoIdentity,
    ) -> Result<PreparedRepoCreation, RepoCatalogError> {
        validate_request_id(lifecycle_request_id)?;
        let store = RepoCatalogStore::open(&self.manager.ledger_dir)?;
        if store.load(repo_id)?.is_some() {
            return Err(RepoCatalogError::AlreadyExists(repo_id));
        }
        match self.manager.catalog_membership.issue(repo_id) {
            Err(CatalogMembershipError::NotMember(_)) => {}
            Err(error) => return Err(error.into()),
            Ok(_) => {
                return Err(RepoCatalogError::CutOutcomeUnknown {
                    repo_id,
                    detail: "process membership exists while durable record is absent".to_string(),
                });
            }
        }
        Ok(PreparedRepoCreation {
            repo_id,
            lifecycle_request_id,
            prepared_identity: identity,
        })
    }

    fn revalidate_creation(
        &self,
        prepared: &PreparedRepoCreation,
    ) -> Result<RevalidatedRepoCreation, RepoCatalogError> {
        let store = RepoCatalogStore::open(&self.manager.ledger_dir)?;
        if let Some(current) = store.load(prepared.repo_id)? {
            if current.confirms_created(prepared.lifecycle_request_id)
                && current.prepared_identity_digest() == prepared.prepared_identity.to_hex()
            {
                return Ok(RevalidatedRepoCreation {
                    repo_id: prepared.repo_id,
                    lifecycle_request_id: prepared.lifecycle_request_id,
                    prepared_identity: prepared.prepared_identity,
                    store,
                });
            }
            return Err(RepoCatalogError::AlreadyExists(prepared.repo_id));
        }
        let observed = prepared_identity::snapshot_initial_primary(self.manager, prepared.repo_id)?;
        self.revalidate_creation_with_identity(prepared, observed)
    }

    fn revalidate_creation_with_identity(
        &self,
        prepared: &PreparedRepoCreation,
        observed: super::PreparedRepoIdentity,
    ) -> Result<RevalidatedRepoCreation, RepoCatalogError> {
        if observed != prepared.prepared_identity {
            return Err(RepoCatalogError::PreparedIdentityChanged(prepared.repo_id));
        }
        Ok(RevalidatedRepoCreation {
            repo_id: prepared.repo_id,
            lifecycle_request_id: prepared.lifecycle_request_id,
            prepared_identity: observed,
            store: RepoCatalogStore::open(&self.manager.ledger_dir)?,
        })
    }

    fn commit_creation(
        &self,
        prepared: &PreparedRepoCreation,
        revalidated: &RevalidatedRepoCreation,
        permit: &RepoCatalogCutPermit,
    ) -> Result<RepoCatalogCreationCommit, RepoCatalogError> {
        validate_creation_revalidation(prepared, revalidated)?;
        self.manager
            .catalog_membership
            .validate_cut_permit(permit, prepared.repo_id)?;
        let _cut = self.manager.catalog_membership.cut_guard()?;
        let _process_lock = revalidated.store.lock()?;
        let desired = RepoCatalogMembershipRecord::normal(
            prepared.repo_id,
            prepared.prepared_identity,
            prepared.lifecycle_request_id,
        );
        match revalidated.store.load(prepared.repo_id)? {
            None => revalidated
                .store
                .publish(&desired)
                .map_err(|failure| publish_error(prepared.repo_id, failure))?,
            Some(current) if current == desired => revalidated.store.seal_visible_record()?,
            Some(_) => return Err(RepoCatalogError::AlreadyExists(prepared.repo_id)),
        }
        let membership = match self.manager.catalog_membership.issue(prepared.repo_id) {
            Ok(existing) => existing,
            Err(CatalogMembershipError::NotMember(_)) => self
                .manager
                .catalog_membership
                .admit_created(prepared.repo_id)
                .map_err(|error| RepoCatalogError::DurableCutProcessStateFailed {
                    repo_id: prepared.repo_id,
                    detail: error.to_string(),
                })?,
            Err(error) => {
                return Err(RepoCatalogError::DurableCutProcessStateFailed {
                    repo_id: prepared.repo_id,
                    detail: error.to_string(),
                });
            }
        };
        Ok(RepoCatalogCreationCommit::new(desired, membership))
    }

    fn prepare_removal(
        &self,
        repo_id: RepoId,
        lifecycle_request_id: Uuid,
        manifest_digest: &str,
    ) -> Result<PreparedRepoRemoval, RepoCatalogError> {
        validate_request_id(lifecycle_request_id)?;
        let store = RepoCatalogStore::open(&self.manager.ledger_dir)?;
        let normal_record = store
            .load(repo_id)?
            .ok_or(RepoCatalogError::NotFound(repo_id))?;
        require_state(&normal_record, RepoCatalogMembershipState::Normal)?;
        let prepared_identity = prepared_identity::snapshot(self.manager, repo_id)?;
        let membership = self.manager.catalog_membership.issue(repo_id)?;
        Ok(PreparedRepoRemoval {
            normal_record,
            membership,
            prepared_identity,
            lifecycle_request_id,
            manifest_digest: manifest_digest.to_string(),
        })
    }

    fn revalidate_removal(
        &self,
        prepared: &PreparedRepoRemoval,
    ) -> Result<RevalidatedRepoRemoval, RepoCatalogError> {
        let repo_id = prepared.normal_record.repo_id;
        let store = RepoCatalogStore::open(&self.manager.ledger_dir)?;
        let desired = RepoCatalogMembershipRecord::removed(
            &prepared.normal_record,
            prepared.lifecycle_request_id,
            &prepared.manifest_digest,
        )?;
        // A lost response or post-replace durability failure is retried from
        // the exact prepared request. Once its Removed record is visible the
        // ordinary authority membership is intentionally gone, so retry must
        // seal that exact durable cut without reopening the database.
        if store.load(repo_id)?.as_ref() == Some(&desired) {
            return Ok(RevalidatedRepoRemoval {
                repo_id,
                lifecycle_request_id: prepared.lifecycle_request_id,
                prepared_identity: prepared.prepared_identity,
                manifest_digest: prepared.manifest_digest.clone(),
                store,
            });
        }
        // A relocation after prepare changes this current removal snapshot, but
        // does not compare against the historical creation digest in record v1.
        let observed = prepared_identity::snapshot(self.manager, repo_id)?;
        if observed != prepared.prepared_identity {
            return Err(RepoCatalogError::PreparedIdentityChanged(repo_id));
        }
        Ok(RevalidatedRepoRemoval {
            repo_id,
            lifecycle_request_id: prepared.lifecycle_request_id,
            prepared_identity: observed,
            manifest_digest: prepared.manifest_digest.clone(),
            store,
        })
    }

    fn commit_removal(
        &self,
        prepared: &PreparedRepoRemoval,
        revalidated: &RevalidatedRepoRemoval,
        permit: &RepoCatalogCutPermit,
    ) -> Result<RepoCatalogRemovalCommit, RepoCatalogError> {
        let repo_id = prepared.normal_record.repo_id;
        validate_removal_revalidation(prepared, revalidated)?;
        self.manager
            .catalog_membership
            .validate_cut_permit(permit, repo_id)?;
        let _cut = self.manager.catalog_membership.cut_guard()?;
        let _process_lock = revalidated.store.lock()?;
        let current = revalidated
            .store
            .load(repo_id)?
            .ok_or(RepoCatalogError::NotFound(repo_id))?;
        let desired = RepoCatalogMembershipRecord::removed(
            &prepared.normal_record,
            prepared.lifecycle_request_id,
            &prepared.manifest_digest,
        )?;
        if current == desired {
            revalidated.store.seal_visible_record()?;
            if !self
                .manager
                .catalog_membership
                .converge_removed(&prepared.membership)?
            {
                return Err(RepoCatalogError::DurableCutProcessStateFailed {
                    repo_id,
                    detail: "durable record is removed but process generation did not converge"
                        .to_string(),
                });
            }
            return Ok(RepoCatalogRemovalCommit::new(
                desired,
                prepared.membership.clone(),
            ));
        }
        if current != prepared.normal_record {
            return Err(RepoCatalogError::CutOutcomeUnknown {
                repo_id,
                detail: "normal membership record changed after prepare".to_string(),
            });
        }
        let revocation = self
            .manager
            .catalog_membership
            .begin_removal(&prepared.membership)?;
        match revalidated.store.publish(&desired) {
            Ok(()) => self
                .manager
                .catalog_membership
                .finalize_removed(&revocation)
                .map_err(|error| RepoCatalogError::DurableCutProcessStateFailed {
                    repo_id,
                    detail: error.to_string(),
                })?,
            Err(failure) if failure.phase == RepoCatalogPublishPhase::BeforeReplace => {
                let abort = self
                    .manager
                    .catalog_membership
                    .abort_removal(&revocation)
                    .err()
                    .map(|error| error.to_string());
                return Err(publish_error_with_abort(repo_id, failure, abort));
            }
            Err(failure) => {
                // The replacement is visible but crash durability is unknown.
                // Leave admission fail-closed in Revoking until retry/restart
                // rereads truth and explicitly seals the directory.
                return Err(publish_error(repo_id, failure));
            }
        }
        Ok(RepoCatalogRemovalCommit::new(
            desired,
            prepared.membership.clone(),
        ))
    }

    fn record(
        &self,
        repo_id: RepoId,
    ) -> Result<Option<RepoCatalogMembershipRecord>, RepoCatalogError> {
        RepoCatalogStore::open(&self.manager.ledger_dir)?.load(repo_id)
    }

    fn retire_removal(
        &self,
        expected: &RepoCatalogMembershipRecord,
        permit: &RepoCatalogCutPermit,
    ) -> Result<RepoCatalogRetirementDisposition, RepoCatalogError> {
        if expected.state() != RepoCatalogMembershipState::Removed
            || expected.removal_manifest_digest().is_none()
        {
            return Err(RepoCatalogError::InvalidRecord(
                "exact retirement requires a manifest-bound Removed tombstone".to_string(),
            ));
        }
        self.manager
            .catalog_membership
            .validate_cut_permit(permit, expected.repo_id())?;
        let store = RepoCatalogStore::open(&self.manager.ledger_dir)?;
        let _cut = self.manager.catalog_membership.cut_guard()?;
        let _process_lock = store.lock()?;
        if store.remove_exact(expected)? {
            Ok(RepoCatalogRetirementDisposition::Retired)
        } else {
            Ok(RepoCatalogRetirementDisposition::AlreadyAbsent)
        }
    }

    fn seed_process_membership(&self) -> Result<(), RepoCatalogError> {
        let store = RepoCatalogStore::open(&self.manager.ledger_dir)?;
        let _cut = self.manager.catalog_membership.cut_guard()?;
        let _process_lock = store.lock()?;
        let records = store.list()?;
        let normal_repo_ids = records
            .iter()
            .filter_map(|record| {
                (record.state() == RepoCatalogMembershipState::Normal).then_some(record.repo_id())
            })
            .collect::<Vec<_>>();
        let removed_repo_ids = records
            .iter()
            .filter_map(|record| {
                (record.state() == RepoCatalogMembershipState::Removed).then_some(record.repo_id())
            })
            .collect::<Vec<_>>();
        self.manager
            .catalog_membership
            .seed_with_removed(normal_repo_ids, removed_repo_ids)?;
        Ok(())
    }

    fn normal_repo_ids(&self) -> Result<Vec<RepoId>, RepoCatalogError> {
        let store = RepoCatalogStore::open(&self.manager.ledger_dir)?;
        let _cut = self.manager.catalog_membership.cut_guard()?;
        let _process_lock = store.lock()?;
        Ok(normal_ids(store.list()?))
    }
}

fn normal_ids(records: Vec<RepoCatalogMembershipRecord>) -> Vec<RepoId> {
    records
        .into_iter()
        .filter_map(|record| {
            (record.state == RepoCatalogMembershipState::Normal).then_some(record.repo_id)
        })
        .collect()
}

fn validate_creation_revalidation(
    prepared: &PreparedRepoCreation,
    revalidated: &RevalidatedRepoCreation,
) -> Result<(), RepoCatalogError> {
    if revalidated.repo_id == prepared.repo_id
        && revalidated.lifecycle_request_id == prepared.lifecycle_request_id
        && revalidated.prepared_identity == prepared.prepared_identity
    {
        Ok(())
    } else {
        Err(RepoCatalogError::PreparedIdentityChanged(prepared.repo_id))
    }
}

fn validate_removal_revalidation(
    prepared: &PreparedRepoRemoval,
    revalidated: &RevalidatedRepoRemoval,
) -> Result<(), RepoCatalogError> {
    let repo_id = prepared.normal_record.repo_id;
    if revalidated.repo_id == repo_id
        && revalidated.lifecycle_request_id == prepared.lifecycle_request_id
        && revalidated.prepared_identity == prepared.prepared_identity
        && revalidated.manifest_digest == prepared.manifest_digest
    {
        Ok(())
    } else {
        Err(RepoCatalogError::PreparedIdentityChanged(repo_id))
    }
}

fn validate_request_id(request_id: Uuid) -> Result<(), RepoCatalogError> {
    if request_id.is_nil() {
        Err(RepoCatalogError::InvalidRecord(
            "lifecycle request_id must not be nil".to_string(),
        ))
    } else {
        Ok(())
    }
}

fn require_state(
    record: &RepoCatalogMembershipRecord,
    expected: RepoCatalogMembershipState,
) -> Result<(), RepoCatalogError> {
    if record.state == expected {
        Ok(())
    } else {
        Err(RepoCatalogError::StateMismatch {
            repo_id: record.repo_id,
            expected: state_name(expected),
            actual: state_name(record.state),
        })
    }
}
